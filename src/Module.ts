import type {
  DefinitionId,
  PrintPart,
  Item,
  ValidateError,
} from "../component-built/interfaces/local-module-module.js";
import {
  LoadedMessageId,
  type ModuleId,
  type MessageId,
  FirstMessageId,
  type MessageToWorker,
  MessageFromWorkerKind,
  type MessageFromWorker,
  MessageToWorkerKind,
} from "./Messages.js";

let nextMessageId: MessageId = FirstMessageId;

async function sendMessage(
  message: MessageToWorker,
  transfers?: Transferable[],
): Promise<MessageFromWorker> {
  console.log(`MAIN: sending ${message.id}, ${message.kind}`);
  let wait = waitForResponse(message.id);
  worker.postMessage(message, transfers ?? []);
  return await wait;
}

async function waitForResponse(id: MessageId): Promise<MessageFromWorker> {
  return new Promise((resolve, reject) => {
    let callback = ({ data }: { data: MessageFromWorker }) => {
      if (data.id !== id) {
        return;
      }
      if (data.kind == MessageFromWorkerKind.Exception) {
        reject(data.exception);
      } else {
        console.log(`MAIN: received ${data.id}, ${data.kind}`);
        resolve(data);
      }
      worker.removeEventListener("message", callback);
    };
    worker.addEventListener("message", callback);
  });
}

const workerUrl = (globalThis as any).__WORKER_URL__;
const workerName = (globalThis as any).__COMPONENT_JS_URL__ ?? "";
// VS Code webviews can't create Workers directly from vscode-resource URLs due to
// cross-origin restrictions. Wrap in a same-origin blob URL that imports the script.
const resolvedWorkerUrl = workerUrl
  ? URL.createObjectURL(
      new Blob([`import ${JSON.stringify(workerUrl)}`], {
        type: "application/javascript",
      }),
    )
  : "./Worker.js";
let worker = new Worker(resolvedWorkerUrl, { type: "module", name: workerName });
console.log(`launched worker: ${workerUrl ?? "./Worker.js"}`);
worker.addEventListener("error", (e) => {
  console.error(`worker error: ${e.message} (${e.filename}:${e.lineno})`);
});
worker.addEventListener("messageerror", (e) => {
  console.error("worker messageerror", e);
});

// Technically a race condition, we should have the event listener installed before launching the worker
let loaded = waitForResponse(LoadedMessageId);

await loaded;
console.log("worker loaded");

let registry = new FinalizationRegistry(async (moduleId: ModuleId) => {
  await sendMessage({
    kind: MessageToWorkerKind.Destroy,
    id: nextMessageId++,
    moduleId,
  });
});

export class Module {
  id: ModuleId;
  items: Item[];
  validateError: ValidateError | null = null;
  printRichCache = new Map<string, PrintPart[]>();

  constructor(
    id: ModuleId,
    items: Item[],
    validateError: ValidateError | null = null,
  ) {
    this.id = id;
    this.items = items;
    this.validateError = validateError;
    registry.register(this, id);
  }

  static async load(source: Uint8Array): Promise<Module> {
    let constructResponse = await sendMessage(
      {
        kind: MessageToWorkerKind.Construct,
        id: nextMessageId++,
        source,
      },
      [source.buffer],
    );
    if (constructResponse.kind !== MessageFromWorkerKind.Construct) {
      throw new Error("unexpected response kind");
    }

    return new Module(
      constructResponse.moduleId,
      constructResponse.items,
      constructResponse.validateError,
    );
  }

  async getSource(): Promise<Uint8Array> {
    let response = await sendMessage({
      kind: MessageToWorkerKind.GetSource,
      id: nextMessageId++,
      moduleId: this.id,
    });
    if (response.kind !== MessageFromWorkerKind.GetSource) {
      throw new Error("unexpected response kind");
    }
    return response.source;
  }

  getCacheKey(id: DefinitionId | undefined): string {
    if (id === undefined) return "module";
    return `${id.tag}:${JSON.stringify(id.val)}`;
  }

  async printRich(id: DefinitionId | undefined): Promise<PrintPart[]> {
    const cacheKey = this.getCacheKey(id);
    const cached = this.printRichCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    let response = await sendMessage({
      kind: MessageToWorkerKind.PrintRich,
      id: nextMessageId++,
      moduleId: this.id,
      definitionId: id,
    });
    if (response.kind !== MessageFromWorkerKind.PrintRich) {
      throw new Error("unexpected response kind");
    }

    this.printRichCache.set(cacheKey, response.result);
    return response.result;
  }

  async printPlain(id: DefinitionId | undefined): Promise<string> {
    let response = await sendMessage({
      kind: MessageToWorkerKind.PrintPlain,
      id: nextMessageId++,
      moduleId: this.id,
      definitionId: id,
    });
    if (response.kind !== MessageFromWorkerKind.PrintPlain) {
      throw new Error("unexpected response kind");
    }
    return response.result;
  }
}
