import net from "node:net";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import * as fs from "node:fs";
import type { ExtensionAPI, ToolDefinition, AgentToolResult, ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { Theme } from "@earendil-works/pi-coding-agent";
import type { ToolRenderContext } from "@earendil-works/pi-coding-agent/core/extensions/types";
import { Text } from "@earendil-works/pi-tui";
import type { Component } from "@earendil-works/pi-tui";
import { Type, Static } from "typebox";

const exec = promisify(execFile);

// ── Schemas ─────────────────────────────────────────────────────────────────

const replSchema = Type.Object({
	code: Type.String({ description: "Python code to execute in the REPL kernel" }),
	name: Type.String({ description: "Kernel name" }),
});

const replManageSchema = Type.Object({
	action: Type.Union([
		Type.Literal("create"),
		Type.Literal("delete"),
		Type.Literal("list"),
		Type.Literal("connect"),
	]),
	name: Type.Optional(Type.String({ description: "Kernel name" })),
	cli: Type.Optional(Type.String({ description: "Path to jupyter_repl_cli.py" })),
	python: Type.Optional(Type.String({ description: "Path to python binary" })),
});

// ── CLI wrapper ─────────────────────────────────────────────────────────────

const DEFAULT_CLI = "~/code/prototyping/replpy_shared/jupyter_repl_cli.py";
const DEFAULT_PYTHON = "~/code/prototyping/replpy_shared/.venv/bin/python";

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
	action: string,
	name: string | undefined,
	cliPath: string,
	pythonPath: string,
): Promise<{ stdout: string; stderr: string }> {
	const args = [action];
	if (name) args.push(name);
	const { stdout, stderr } = await exec(resolvePath(pythonPath), [resolvePath(cliPath), ...args]);
	return { stdout: stdout.trim(), stderr: stderr.trim() };
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

function sendToKernel(socketPath: string, code: string): Promise<Record<string, any>> {
	return new Promise((resolve, reject) => {
		const sock = new net.Socket();
		const timeout = setTimeout(() => {
			sock.destroy();
			reject(new Error("REPL socket connection timed out"));
		}, 30_000);

		sock.on("error", (err) => {
			clearTimeout(timeout);
			reject(err);
		});

		sock.on("data", (chunk) => {
			try {
				resolve(JSON.parse(chunk.toString()));
			} catch {
				reject(new Error("Invalid JSON from kernel: " + chunk.toString().slice(0, 200)));
			}
			clearTimeout(timeout);
			sock.destroy();
		});

		sock.connect(socketPath, () => {
			sock.write(JSON.stringify({ code }));
			sock.end();
		});
	});
}


// ── Tools ───────────────────────────────────────────────────────────────────

const replTool: ToolDefinition = {
	name: "repl",
	label: "Repl",
	description: "Execute Python code in a persistent REPL kernel. State (variables, imports) persists across calls. Returns {ok, result, stdout, error}.",
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
		_signal,
		_onUpdate,
		_ctx: ExtensionContext,
	): Promise<AgentToolResult<Record<string, any>>> {
		const target = params.name;
		const socketPath = getSocketPath(target);
		if (!socketPath) {
			return { content: [{ type: "text", text: `Cannot find socket for kernel '${target}'. Kernel may be dead.` }], isError: true };
		}
		try {
			_onUpdate?.({ content: [{ type: "text", text: `repl: ${target}` }] });
			const result = await sendToKernel(socketPath, params.code);
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
				details: result,
			};
		} catch (err: any) {
			return { content: [{ type: "text", text: err.message }], details: undefined, isError: true };
		}
	},
};

const replManageTool: ToolDefinition = {
	name: "repl-manage",
	label: "Repl Manage",
	description: "Manage REPL kernel lifecycle. Actions: create (start kernel), delete (shutdown), list (show kernels), connect (print connection JSON).",
	parameters: replManageSchema,
	async execute(
		_toolCallId,
		params: Static<typeof replManageSchema>,
		_signal,
		_onUpdate,
		_ctx: ExtensionContext,
	): Promise<AgentToolResult<string>> {
		const cliPath = params.cli ?? DEFAULT_CLI;
		const pythonPath = params.python ?? DEFAULT_PYTHON;
		const name = params.name ?? (params.action === "create" ? generateKernelName() : undefined);
		try {
			const { stdout, stderr } = await runCli(params.action, name, cliPath, pythonPath);
			const text = stderr ? `${stdout}\nstderr: ${stderr}` : stdout;
			return { content: [{ type: "text", text }], details: stdout };
		} catch (err: any) {
			return { content: [{ type: "text", text: err.message }], details: undefined, isError: true };
		}
	},
};

// ── Extension ───────────────────────────────────────────────────────────────

export default function (pi: ExtensionAPI): void {
	pi.registerTool(replTool);
	pi.registerTool(replManageTool);
}