import net from "node:net";
import * as fs from "node:fs";
import type { ExtensionAPI, ToolDefinition, AgentToolResult, ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { Theme } from "@earendil-works/pi-coding-agent";
import type { ToolRenderContext } from "@earendil-works/pi-coding-agent/core/extensions/types";
import { Text } from "@earendil-works/pi-tui";
import type { Component } from "@earendil-works/pi-tui";
import { Type, Static } from "typebox";

// BEGIN STANDALONE TRANSPORT
// Keep transport inline: this extension is deployed as a single copied/symlinked file.
export const REQUEST_TIMEOUT_MS = 300_000;
const INTERRUPT_TIMEOUT_MS = 1_000;

type Json = Record<string, any>;
type CliResult = { stdout: string; stderr: string; code: number; killed?: boolean };
export type ExecCli = (args: string[], options: { signal?: AbortSignal; timeout: number }) => Promise<CliResult>;

class BrokerUnavailableError extends Error {}
class RequestAbortedError extends Error {
	constructor() {
		super("REPL request aborted");
		this.name = "AbortError";
	}
}

/** Disconnecting cancels only this wait, not code already submitted to a kernel. */
export function sendJson(
	socketPath: string,
	payload: unknown,
	options: { signal?: AbortSignal; timeout?: number; onDispatch?: () => void } = {},
): Promise<Json> {
	return new Promise((resolve, reject) => {
		const { signal } = options;
		if (signal?.aborted) return reject(new RequestAbortedError());
		const sock = new net.Socket();
		const chunks: Buffer[] = [];
		let settled = false;
		const finish = (error?: Error, result?: Json) => {
			if (settled) return;
			settled = true;
			clearTimeout(timeout);
			signal?.removeEventListener("abort", abort);
			sock.destroy();
			if (error) reject(error);
			else resolve(result!);
		};
		const abort = () => finish(new RequestAbortedError());
		const timeout = setTimeout(() => finish(new Error(`Unix socket request timed out: ${socketPath}`)),
			options.timeout ?? REQUEST_TIMEOUT_MS);
		signal?.addEventListener("abort", abort, { once: true });
		sock.on("error", (error) => finish(error));
		sock.on("data", (chunk: Buffer) => chunks.push(chunk));
		sock.on("end", () => {
			const body = Buffer.concat(chunks).toString();
			try {
				finish(undefined, JSON.parse(body));
			} catch {
				finish(new Error(`Invalid JSON from ${socketPath}: ${body.slice(0, 200)}`));
			}
		});
		sock.on("close", () => finish(new Error(`Unix socket closed before a response: ${socketPath}`)));
		try {
			sock.connect(socketPath, () => {
				if (settled) return;
				try {
					const body = JSON.stringify(payload);
					options.onDispatch?.();
					sock.end(body);
				} catch (error) {
					finish(error instanceof Error ? error : new Error(String(error)));
				}
			});
		} catch (error) {
			finish(error instanceof Error ? error : new Error(String(error)));
		}
	});
}

function brokerPayload(operation: Json): Json {
	return { operation, kernel_dir: null, python: null, kernel_script: null };
}

async function brokerRequest(socketPath: string, operation: Json, options: Parameters<typeof sendJson>[2]): Promise<Json> {
	let wire: Json;
	try {
		wire = await sendJson(socketPath, brokerPayload(operation), options);
	} catch (error: any) {
		if (error?.code === "ENOENT" || error?.code === "ECONNREFUSED") throw new BrokerUnavailableError();
		throw error;
	}
	if (!wire?.ok) throw new Error(wire?.error || "Replmux broker request failed");
	return wire.response;
}

function executeCli(execCli: ExecCli, args: string[], signal?: AbortSignal): Promise<CliResult> {
	return new Promise((resolve, reject) => {
		if (signal?.aborted) return reject(new RequestAbortedError());
		let settled = false;
		const finish = (error?: unknown, result?: CliResult) => {
			if (settled) return;
			settled = true;
			signal?.removeEventListener("abort", abort);
			if (error) reject(error);
			else resolve(result!);
		};
		const abort = () => finish(new RequestAbortedError());
		signal?.addEventListener("abort", abort, { once: true });
		// Do not wait for a slow client process to exit before attempting interruption.
		try {
			execCli(args, { signal, timeout: REQUEST_TIMEOUT_MS }).then(
				(result) => finish(undefined, result), (error) => finish(error),
			);
		} catch (error) {
			finish(error);
		}
	});
}

function parseCli(result: CliResult): Json {
	if (result.killed || result.code !== 0) {
		throw new Error(result.stderr.trim() || result.stdout.trim() || "Replmux CLI stopped without a successful response");
	}
	return JSON.parse(result.stdout);
}

/** A separate deadline, never the already-aborted execution signal. */
async function interruptCli(execCli: ExecCli, name: string): Promise<Json> {
	const controller = new AbortController();
	let timer: ReturnType<typeof setTimeout> | undefined;
	try {
		return await Promise.race([
			execCli(["--transport", "local", "--json", "kernel", "interrupt", name], {
				signal: controller.signal, timeout: INTERRUPT_TIMEOUT_MS,
			}).then(parseCli),
			new Promise<never>((_, reject) => {
				timer = setTimeout(() => {
					reject(new Error("Kernel interrupt request timed out"));
					controller.abort();
				}, INTERRUPT_TIMEOUT_MS);
			}),
		]);
	} finally {
		clearTimeout(timer);
	}
}

export async function executeRepl(options: {
	name: string;
	code: string;
	signal?: AbortSignal;
	brokerSocket?: string;
	getKernelSocket: () => string | null;
	execCli: ExecCli;
}): Promise<{ result: Json; transport: string }> {
	const { name, code, signal, brokerSocket, getKernelSocket, execCli } = options;
	if (signal?.aborted) throw new Error("REPL wait aborted before submission; no code sent.");
	let transport = brokerSocket ? "broker" : "kernel";
	let dispatched = false;
	const requestOptions = { signal, onDispatch: () => { dispatched = true; } };
	try {
		if (brokerSocket) {
			try {
				const response = await brokerRequest(brokerSocket, { action: "exec", name, code }, requestOptions);
				if (response?.type !== "executed" || !response.response) throw new Error("Invalid broker execution response");
				return { result: response.response, transport };
			} catch (error) {
				// Never retry execution that might already have reached the kernel.
				if (signal?.aborted || dispatched || !(error instanceof BrokerUnavailableError)) throw error;
			}
		}
		const socketPath = getKernelSocket();
		transport = socketPath ? "kernel" : "cli-jupyter";
		if (signal?.aborted) throw new RequestAbortedError();
		if (socketPath) return { result: await sendJson(socketPath, { code }, requestOptions), transport };
		dispatched = true;
		const result = await executeCli(execCli, ["--transport", "local", "--json", "kernel", "exec", name, code], signal);
		if (signal?.aborted) throw new RequestAbortedError();
		return { result: parseCli(result), transport };
	} catch (error) {
		if (!signal?.aborted) throw error;
		if (!dispatched) throw new Error("REPL wait aborted before submission; no code sent.");
		// Older minimal workers falsely acknowledge interruption. Never use that ack
		// as evidence, or send SIGINT (which shuts them down).
		if (transport === "kernel" || getKernelSocket()) {
			throw new Error("REPL wait aborted. Minimal-worker interruption is unsupported; code may still be running. Use a standard kernelspec for interruptible execution.");
		}
		let delivery: string;
		try {
			const reply = transport === "broker"
				? await brokerRequest(brokerSocket!, { action: "interrupt", name }, { timeout: INTERRUPT_TIMEOUT_MS })
				: await interruptCli(execCli, name);
			const signalSent = reply?.type === "interrupt_signal_sent" || reply?.status === "signal_sent";
			const message = reply?.type === "jupyter_reply" ? reply.message : reply;
			const acknowledged = message?.header?.msg_type === "interrupt_reply" && message?.content?.status === "ok";
			delivery = signalSent ? "SIGINT sent" : acknowledged ? "interrupt acknowledged" : "interrupt response unrecognized";
		} catch (interruptError) {
			throw new Error(`REPL wait aborted; kernel interrupt failed: ${interruptError instanceof Error ? interruptError.message : String(interruptError)}. Code may still be running.`);
		}
		throw new Error(`REPL wait aborted; ${delivery}. Cancellation and retained state are unconfirmed; code may still be running.`);
	}
}
// END STANDALONE TRANSPORT

// ── Schemas ─────────────────────────────────────────────────────────────────

const replSchema = Type.Object({
	code: Type.String({ description: "Python code to execute in the REPL kernel. Single expressions return a value; statements do not." }),
	name: Type.String({ description: "Name of a running kernel (created via repl-manage)" }),
});

const replManageSchema = Type.Object({
	action: Type.Union([
		Type.Literal("create"),
		Type.Literal("delete"),
		Type.Literal("list"),
		Type.Literal("connect"),
	]),
	name: Type.Optional(Type.String({ description: "[optional] Kernel name. Auto-generated on create if omitted." })),
	kernelspec: Type.Optional(Type.String({ description: "For create: installed kernelspec name or kernel.json path. Use an ipykernel spec for interruptible Python; the default minimal worker cannot cancel code non-destructively." })),
	binary: Type.Optional(Type.String({ description: "[optional] Path to the Rust replmux binary" })),
});

// ── CLI wrapper ─────────────────────────────────────────────────────────────

const DEFAULT_BINARY = process.env.REPLMUX_BINARY ?? (
	process.platform === "win32"
		? `${process.env.USERPROFILE ?? process.env.HOME ?? "."}\\.cargo\\bin\\replmux.exe`
		: "~/.local/bin/replmux"
);
const DEFAULT_BROKER_SOCKET = process.env.REPLMUX_BROKER_SOCKET ?? "~/.replmux/b.sock";

function resolvePath(p: string): string {
	return p.replace(/^~/, process.env.HOME ?? "");
}

function generateKernelName(): string {
	const dir = process.cwd().split("/").pop() ?? "repl";
	const prefix = dir.split(/[-_]/).map(s => s[0]).join("").slice(0, 3);
	const prefix2 = prefix.length < 2 ? dir.slice(0, 3) : prefix;
	const ts = new Date().toISOString().replace(/[^0-9]/g, "").slice(0, 14);
	return `${prefix2}-${ts}`;
}

async function runCli(
	pi: ExtensionAPI,
	action: string,
	name: string | undefined,
	binaryPath: string,
	signal: AbortSignal | undefined,
	kernelspec?: string,
): Promise<{ stdout: string; stderr: string }> {
	if (signal?.aborted) throw new Error("Replmux management request aborted before submission.");
	if (kernelspec && action !== "create") throw new Error("kernelspec is only supported for create");
	const args = ["kernel", action];
	if (name) args.push(name);
	if (kernelspec) args.push("--kernelspec", kernelspec);
	const result = await pi.exec(resolvePath(binaryPath), args, { signal, timeout: REQUEST_TIMEOUT_MS });
	const stdout = result.stdout.trim();
	const stderr = result.stderr.trim();
	if (signal?.aborted || result.killed) throw new Error("Replmux management request aborted; lifecycle outcome is unconfirmed.");
	if (result.code !== 0) {
		throw new Error(stderr || stdout || `replmux exited with code ${result.code}`);
	}
	return { stdout, stderr };
}

function getSocketPath(kernelName: string): string | null {
	const kernelDir = resolvePath(process.env.REPLMUX_KERNEL_DIR ?? "~/.jupyter-repl/kernels");
	const connPath = `${kernelDir}/${kernelName}.json`;
	try {
		const conn = JSON.parse(fs.readFileSync(connPath, "utf8"));
		return typeof conn.socket_path === "string" && conn.socket_path ? conn.socket_path : null;
	} catch {
		return null;
	}
}

// ── Tools ───────────────────────────────────────────────────────────────────

function createReplTool(pi: ExtensionAPI): ToolDefinition {
	return {
	name: "repl",
	label: "Repl",
	promptSnippet: "Execute code in a persistent REPL kernel. If a kernel is already running (created here or shared by another agent) you can reuse it; otherwise create one with repl-manage (action: create). State (variables, imports) persists across calls. Single expressions return a value; statements do not. Skill contains numerous available language kernels.",
	description: "Execute code in a persistent REPL kernel. If a kernel is already running (created here or shared by another agent) you can reuse it; otherwise create one with repl-manage (action: create). State (variables, imports) persists across calls. Single expressions return a value; statements do not. Skill contains numerous available language kernels.",
	parameters: replSchema,
	renderCall(args: Static<typeof replSchema>, theme: Theme, context: ToolRenderContext) {
		const text = (context.lastComponent as Text | undefined) ?? new Text("", 0, 0);
		const name = args.name || "";
		const code = args.code ? args.code.split("\n").map((l: string, i: number) => i === 0 ? `>>> ${l}` : `... ${l}`).join("\n") : "";
		text.setText(`${theme.fg("toolTitle", theme.bold(`repl: ${name}`))}${code ? "\n" + code : ""}`);
		return text;
	},
	renderResult(result: AgentToolResult<any>, options: { expanded: boolean; isPartial: boolean }, theme: Theme, context: ToolRenderContext) {
		const text = (context.lastComponent as Text | undefined) ?? new Text("", 0, 0);
		const content = result.content.map((c: any) => c.text || "").join("\n");
		const colorFn = result.isError ? theme.fg("toolError", content) : theme.fg("toolOutput", content);
		text.setText(colorFn || content);
		return text;
	},
	async execute(
		_toolCallId,
		params: Static<typeof replSchema>,
		signal,
		_onUpdate,
		_ctx: ExtensionContext,
	): Promise<AgentToolResult<Record<string, any>>> {
		const target = params.name;
		try {
			_onUpdate?.({ content: [{ type: "text", text: `repl: ${target}` }] });
			const { result, transport } = await executeRepl({
				name: target,
				code: params.code,
				signal,
				brokerSocket: process.platform === "win32" ? undefined : resolvePath(DEFAULT_BROKER_SOCKET),
				getKernelSocket: () => getSocketPath(target),
				execCli: (args, options) => pi.exec(resolvePath(DEFAULT_BINARY), args, options),
			});
			let resultText = "";
			if (!result.ok) {
				resultText = `  ✗ ${result.error}`;
			} else if (result.mode === "eval" && result.result !== null) {
				resultText = `  → ${result.result}`;
			}
			if (result.stdout) resultText += `\n  stdout: ${result.stdout.trim()}`;
			if (result.stderr) resultText += `\n  stderr: ${result.stderr.trim()}`;
			return {
				content: [{ type: "text", text: resultText || "(ok)" }],
				details: { ...result, transport },
			};
		} catch (err: unknown) {
			// Pi marks thrown tool errors as failures; a returned isError is ignored.
			throw err instanceof Error ? err : new Error(String(err));
		}
	},
	};
}

function createReplManageTool(pi: ExtensionAPI): ToolDefinition {
	return {
		name: "repl-manage",
		label: "Repl Manage",
		promptSnippet: "Manage REPL jupyter-compatible kernel lifecycle for numerous languages and runtimes. create (start kernel, name is auto-generated if omitted), list (show kernels), connect (print connection JSON), delete (shutdown).",
		description: "Manage REPL jupyter-compatible kernel lifecycle for numerous languages and runtimes. create (start kernel, name is auto-generated if omitted), list (show kernels), connect (print connection JSON), delete (shutdown).",
		parameters: replManageSchema,
		async execute(
		_toolCallId,
		params: Static<typeof replManageSchema>,
		signal,
		_onUpdate,
		_ctx: ExtensionContext,
	): Promise<AgentToolResult<string>> {
		const binaryPath = params.binary ?? DEFAULT_BINARY;
		const name = params.name ?? (params.action === "create" ? generateKernelName() : undefined);
		try {
			const { stdout, stderr } = await runCli(pi, params.action, name, binaryPath, signal, params.kernelspec);
			const text = stderr ? `${stdout}\nstderr: ${stderr}` : stdout;
			return { content: [{ type: "text", text }], details: stdout };
		} catch (err: unknown) {
			throw err instanceof Error ? err : new Error(String(err));
		}
		},
	};
}

// ── Extension ───────────────────────────────────────────────────────────────

export default function (pi: ExtensionAPI): void {
	pi.registerFlag("repl", {
		description: "Start with all active tools except bash, plus the REPL tools",
		type: "boolean",
		default: false,
	});

	pi.registerTool(createReplTool(pi));
	pi.registerTool(createReplManageTool(pi));

	pi.on("session_start", () => {
		if (pi.getFlag("repl") === true) {
			const tools = pi.getActiveTools().filter((name) => name !== "bash");
			for (const name of ["repl", "repl-manage"]) {
				if (!tools.includes(name)) tools.push(name);
			}
			pi.setActiveTools(tools);
		}
	});
}
