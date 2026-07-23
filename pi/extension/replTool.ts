import net from "node:net";
import * as fs from "node:fs";
import type { ExtensionAPI, ToolDefinition, AgentToolResult, ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { Theme } from "@earendil-works/pi-coding-agent";
import type { ToolRenderContext } from "@earendil-works/pi-coding-agent/core/extensions/types";
import { Text } from "@earendil-works/pi-tui";
import type { Component } from "@earendil-works/pi-tui";
import { Type, Static } from "typebox";

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
	binary: Type.Optional(Type.String({ description: "[optional] Path to the Rust multirepl binary" })),
});

// ── CLI wrapper ─────────────────────────────────────────────────────────────

const DEFAULT_BINARY = process.env.MULTIREPL_BINARY ?? "~/code/multirepl/cli/target/release/multirepl";
const DEFAULT_BROKER_SOCKET = process.env.MULTIREPL_BROKER_SOCKET ?? "~/.multirepl/b.sock";

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
): Promise<{ stdout: string; stderr: string }> {
	const args = [action];
	if (name) args.push(name);
	const result = await pi.exec(resolvePath(binaryPath), args, { signal, timeout: 30_000 });
	const stdout = result.stdout.trim();
	const stderr = result.stderr.trim();
	if (result.code !== 0) {
		throw new Error(stderr || stdout || `multirepl exited with code ${result.code}`);
	}
	return { stdout, stderr };
}

function getSocketPath(kernelName: string): string | null {
	const connPath = `${process.env.HOME}/.jupyter-repl/kernels/${kernelName}.json`;
	try {
		const conn = JSON.parse(fs.readFileSync(connPath, "utf8"));
		return conn.socket_path ?? null;
	} catch {
		return null;
	}
}

class BrokerUnavailableError extends Error {}

function sendJson(socketPath: string, payload: unknown): Promise<Record<string, any>> {
	return new Promise((resolve, reject) => {
		const sock = new net.Socket();
		const chunks: Buffer[] = [];
		const timeout = setTimeout(() => {
			sock.destroy();
			reject(new Error(`Unix socket request timed out: ${socketPath}`));
		}, 30_000);

		const finish = (callback: () => void) => {
			clearTimeout(timeout);
			callback();
		};
		sock.on("error", (error) => finish(() => reject(error)));
		sock.on("data", (chunk: Buffer) => chunks.push(chunk));
		sock.on("end", () => finish(() => {
			const body = Buffer.concat(chunks).toString();
			try {
				resolve(JSON.parse(body));
			} catch {
				reject(new Error(`Invalid JSON from ${socketPath}: ${body.slice(0, 200)}`));
			}
		}));
		sock.connect(socketPath, () => {
			sock.end(JSON.stringify(payload));
		});
	});
}

function sendToKernel(socketPath: string, code: string): Promise<Record<string, any>> {
	return sendJson(socketPath, { code });
}

async function sendToBroker(kernelName: string, code: string): Promise<Record<string, any>> {
	const socketPath = resolvePath(DEFAULT_BROKER_SOCKET);
	let wireResponse: Record<string, any>;
	try {
		wireResponse = await sendJson(socketPath, {
			operation: { action: "exec", name: kernelName, code },
			kernel_dir: null,
			python: null,
			kernel_script: null,
		});
	} catch (error: any) {
		if (error?.code === "ENOENT" || error?.code === "ECONNREFUSED") {
			throw new BrokerUnavailableError();
		}
		throw error;
	}
	if (!wireResponse.ok) {
		throw new Error(wireResponse.error || "Multirepl broker request failed");
	}
	if (wireResponse.response?.type !== "executed" || !wireResponse.response.response) {
		throw new Error("Multirepl broker returned an invalid execution response");
	}
	return wireResponse.response.response;
}

async function executeViaCli(
	pi: ExtensionAPI,
	kernelName: string,
	code: string,
	signal: AbortSignal | undefined,
): Promise<Record<string, any>> {
	const result = await pi.exec(resolvePath(DEFAULT_BINARY), ["--json", "exec", kernelName, code], {
		signal,
		timeout: 30_000,
	});
	if (result.code !== 0) {
		throw new Error(result.stderr.trim() || result.stdout.trim() || `multirepl exited with code ${result.code}`);
	}
	try {
		return JSON.parse(result.stdout);
	} catch {
		throw new Error(`Invalid JSON from multirepl: ${result.stdout.slice(0, 200)}`);
	}
}

// ── Tools ───────────────────────────────────────────────────────────────────

function createReplTool(pi: ExtensionAPI): ToolDefinition {
	return {
	name: "repl",
	label: "Repl",
	description: "Execute Python code in a persistent REPL kernel. If a kernel is already running (created here or shared by another agent) you can reuse it; otherwise create one with repl-manage (action: create). State (variables, imports) persists across calls. Single expressions return a value; statements do not.",
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
			let transport = "broker";
			let result: Record<string, any>;
			try {
				result = await sendToBroker(target, params.code);
			} catch (error) {
				if (!(error instanceof BrokerUnavailableError)) throw error;
				transport = "kernel";
				const socketPath = getSocketPath(target);
				if (socketPath) {
					result = await sendToKernel(socketPath, params.code);
				} else {
					transport = "cli-jupyter";
					result = await executeViaCli(pi, target, params.code, signal);
				}
			}
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
		} catch (err: any) {
			return { content: [{ type: "text", text: err.message }], details: undefined, isError: true };
		}
	},
	};
}

function createReplManageTool(pi: ExtensionAPI): ToolDefinition {
	return {
		name: "repl-manage",
		label: "Repl Manage",
		description: "Manage REPL kernel lifecycle. create (start kernel, name is auto-generated if omitted), list (show kernels), connect (print connection JSON), delete (shutdown).",
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
			const { stdout, stderr } = await runCli(pi, params.action, name, binaryPath, signal);
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
	pi.registerTool(createReplTool(pi));
	pi.registerTool(createReplManageTool(pi));
}
