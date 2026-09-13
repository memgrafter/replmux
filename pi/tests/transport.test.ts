import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { stripTypeScriptTypes } from "node:module";
import net from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test, type TestContext } from "node:test";
import { getEventListeners } from "node:events";
import type { ExecCli } from "../extension/replTool.ts";

// Exercise the actual inline transport without loading Pi's UI packages. No
// duplicate implementation or sidecar is needed by a copied/symlinked extension.
const source = await readFile(new URL("../extension/replTool.ts", import.meta.url), "utf8");
const transport = source.split("// BEGIN STANDALONE TRANSPORT\n")[1]?.split("// END STANDALONE TRANSPORT")[0];
assert.ok(transport, "single-file extension must contain the transport section");
const javascript = stripTypeScriptTypes(`import net from "node:net";\n${transport}`);
const { executeRepl, sendJson } = await import(`data:text/javascript;base64,${Buffer.from(javascript).toString("base64")}`) as typeof import("../extension/replTool.ts");

test("extension has no relative module dependencies", () => {
	assert.doesNotMatch(source, /(?:from\s*|import\s*\(?\s*)["']\.\.?\//);
});

const noCli: ExecCli = async () => { throw new Error("Unexpected CLI fallback"); };
const noSocket = () => null;
const base = { name: "test", code: "sleep", getKernelSocket: noSocket, execCli: noCli };

async function fixture(t: TestContext, handle: (request: any, socket: net.Socket) => void) {
	// macOS Unix socket paths are limited to 104 bytes.
	const dir = await mkdtemp(join(process.platform === "darwin" ? "/tmp" : tmpdir(), "rmx-pi-"));
	const path = join(dir, "s");
	const sockets = new Set<net.Socket>();
	const server = net.createServer({ allowHalfOpen: true }, (socket) => {
		sockets.add(socket);
		socket.on("error", () => {}); // Expected when an aborted client disconnects.
		socket.on("close", () => sockets.delete(socket));
		let body = "";
		socket.on("data", (chunk) => { body += chunk; });
		socket.on("end", () => {
			if (body) handle(JSON.parse(body), socket);
			else socket.end(); // Abort may close a connection before dispatch.
		});
	});
	t.after(async () => {
		for (const socket of sockets) socket.destroy();
		await new Promise<void>((resolve) => server.close(() => resolve()));
		await rm(dir, { recursive: true, force: true });
	});
	await new Promise<void>((resolve, reject) => {
		server.once("error", reject);
		server.listen(path, resolve);
	});
	return path;
}

function reply(socket: net.Socket, response: any) {
	socket.end(JSON.stringify({ ok: true, response }));
}

const signalReply = { type: "interrupt_signal_sent", pid: 123 };
const messageReply = { type: "jupyter_reply", message: { header: { msg_type: "interrupt_reply" }, content: { status: "ok" } } };

for (const [label, controlReply, expected] of [
	["signal", signalReply, /SIGINT sent/],
	["message", messageReply, /interrupt acknowledged/],
] as const) {
	test(`broker ${label}: abort interrupts on a separate connection, retains fixture state`, { timeout: 5000 }, async (t) => {
		const busy = Promise.withResolvers<void>();
		const controller = new AbortController();
		const operations: string[] = [];
		let answer: number | undefined;
		let running = false;
		const path = await fixture(t, (req, socket) => {
			const op = req.operation;
			operations.push(op.action);
			if (op.action === "interrupt") {
				running = false;
				reply(socket, controlReply);
			} else if (op.code === "sleep") {
				running = true;
				busy.resolve(); // Execution response intentionally withheld.
			} else {
				if (op.code === "set") answer = 42;
				reply(socket, { type: "executed", response: { ok: true, result: answer } });
			}
		});
		const options = { ...base, brokerSocket: path };
		await executeRepl({ ...options, code: "set" });
		const pending = executeRepl({ ...options, signal: controller.signal });
		const rejected = assert.rejects(pending, (error: Error) => {
			assert.match(error.message, expected);
			assert.match(error.message, /Cancellation and retained state are unconfirmed/);
			return true;
		});
		await busy.promise;
		assert.equal(running, true);
		controller.abort();
		controller.abort();
		await rejected;
		assert.equal(running, false);
		assert.equal((await executeRepl({ ...options, code: "read" })).result.result, 42);
		assert.deepEqual(operations, ["exec", "exec", "interrupt", "exec"]);
		assert.equal(getEventListeners(controller.signal, "abort").length, 0);
	});
}

test("pre-aborted call submits neither execution nor interrupt", async () => {
	const controller = new AbortController();
	controller.abort();
	await assert.rejects(executeRepl({ ...base, signal: controller.signal }), /before submission/);
	await assert.rejects(sendJson("/not-used", {}, { signal: controller.signal }), /aborted/);
});

test("abort before socket dispatch sends no request", { timeout: 5000 }, async (t) => {
	let requests = 0;
	const path = await fixture(t, () => { requests++; });
	const controller = new AbortController();
	const pending = executeRepl({ ...base, brokerSocket: path, signal: controller.signal });
	controller.abort();
	await assert.rejects(pending, /before submission/);
	assert.equal(requests, 0);
});

for (const useBroker of [false, true]) {
	test(`minimal worker abort is explicit without destructive escalation (broker=${useBroker})`, { timeout: 5000 }, async (t) => {
		const busy = Promise.withResolvers<void>();
		let requests = 0;
		const path = await fixture(t, () => { requests++; busy.resolve(); });
		const controller = new AbortController();
		const pending = executeRepl({
			...base, brokerSocket: useBroker ? path : undefined,
			getKernelSocket: () => path, signal: controller.signal,
		});
		const rejected = assert.rejects(pending, /Minimal-worker interruption is unsupported; code may still be running/);
		await busy.promise;
		controller.abort();
		await rejected;
		assert.equal(requests, 1);
	});
}

test("missing broker falls back once; CLI abort uses a fresh control signal", { timeout: 5000 }, async (t) => {
	const path = await fixture(t, () => { throw new Error("Wrong socket"); });
	const busy = Promise.withResolvers<void>();
	const controller = new AbortController();
	const calls: string[][] = [];
	const execCli: ExecCli = async (args, options) => {
		calls.push(args);
		assert.equal(args[1], "local");
		if (args.includes("interrupt")) {
			assert.notEqual(options.signal, controller.signal);
			assert.equal(options.signal?.aborted, false);
			return { code: 0, stdout: JSON.stringify({ status: "signal_sent" }), stderr: "" };
		}
		busy.resolve();
		// Simulate a slow CLI which doesn't settle on abort. Pi must not wait for it.
		return new Promise(() => {});
	};
	const pending = executeRepl({ ...base, brokerSocket: `${path}-missing`, execCli, signal: controller.signal });
	const rejected = assert.rejects(pending, /SIGINT sent/);
	await busy.promise;
	controller.abort();
	await rejected;
	assert.equal(calls.length, 2);
	assert.equal(getEventListeners(controller.signal, "abort").length, 0);
});

for (const failure of ["hang", "error"]) {
	test(`broker interrupt ${failure} is bounded and never retried through CLI`, { timeout: 5000 }, async (t) => {
		const busy = Promise.withResolvers<void>();
		const controller = new AbortController();
		const path = await fixture(t, (req, socket) => {
			if (req.operation.action === "exec") busy.resolve();
			else if (failure === "error") socket.end(JSON.stringify({ ok: false, error: "kernel identity mismatch" }));
		});
		const pending = executeRepl({ ...base, brokerSocket: path, signal: controller.signal });
		const rejected = assert.rejects(pending, failure === "hang" ? /interrupt failed:.*timed out/ : /identity mismatch/);
		await busy.promise;
		controller.abort();
		await rejected;
	});
}

test("CLI interrupt deadline bounds even an uncooperative client", { timeout: 5000 }, async () => {
	const busy = Promise.withResolvers<void>();
	const controller = new AbortController();
	let controlSignal: AbortSignal | undefined;
	const execCli: ExecCli = async (args, options) => {
		if (args.includes("interrupt")) controlSignal = options.signal;
		else busy.resolve();
		return new Promise(() => {});
	};
	const pending = executeRepl({ ...base, execCli, signal: controller.signal });
	const rejected = assert.rejects(pending, /interrupt failed: Kernel interrupt request timed out/);
	await busy.promise;
	controller.abort();
	await rejected;
	assert.equal(controlSignal?.aborted, true);
});

test("normal completion removes abort listener; later abort does not interrupt", { timeout: 5000 }, async (t) => {
	let requests = 0;
	const path = await fixture(t, (_, socket) => {
		requests++;
		reply(socket, { type: "executed", response: { ok: true, result: 42 } });
	});
	const controller = new AbortController();
	const result = await executeRepl({ ...base, brokerSocket: path, signal: controller.signal });
	assert.equal(result.result.result, 42);
	assert.equal(getEventListeners(controller.signal, "abort").length, 0);
	controller.abort();
	assert.equal(requests, 1);
});

test("malformed broker reply is not retried", { timeout: 5000 }, async (t) => {
	const path = await fixture(t, (_, socket) => socket.end("not json"));
	await assert.rejects(executeRepl({ ...base, brokerSocket: path }), /Invalid JSON/);
});

test("socket timeout removes abort listener", { timeout: 5000 }, async (t) => {
	const path = await fixture(t, () => {});
	const controller = new AbortController();
	await assert.rejects(sendJson(path, {}, { signal: controller.signal, timeout: 20 }), /timed out/);
	assert.equal(getEventListeners(controller.signal, "abort").length, 0);
});
