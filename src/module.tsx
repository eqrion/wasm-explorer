import type {
  Range,
  PrintPart,
  Item,
} from "../component-built/interfaces/local-module-module.d.ts";
import {
  LoadedMessageId,
  type ModuleId,
  type MessageId,
  FirstMessageId,
  type MessageToWorker,
  MessageFromWorkerKind,
  type MessageFromWorker,
  MessageToWorkerKind,
} from "./messages.js";

let nextMessageId: MessageId = FirstMessageId;

async function sendMessage(
  message: MessageToWorker,
): Promise<MessageFromWorker> {
  console.log(`MAIN: sending ${message.id}, ${message.kind}`);
  let wait = waitForResponse(message.id);
  worker.postMessage(message);
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

let worker = new Worker("./worker.js", { type: "module" });
console.log("launched worker");

// Technically a race condition, we should have the event listener installed before launching the worker
let loaded = waitForResponse(LoadedMessageId);

await loaded;

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
  printRichCache = new Map<string, PrintPart[]>();

  constructor(id: ModuleId, items: Item[]) {
    this.id = id;
    this.items = items;
    registry.register(this, id);
  }

  static async load(source: Uint8Array): Promise<Module> {
    let constructResponse = await sendMessage({
      kind: MessageToWorkerKind.Construct,
      id: nextMessageId++,
      source,
    });
    if (constructResponse.kind !== MessageFromWorkerKind.Construct) {
      throw new Error("unexpected response kind");
    }

    let itemsResponse = await sendMessage({
      kind: MessageToWorkerKind.Items,
      id: nextMessageId++,
      moduleId: constructResponse.moduleId,
    });
    if (itemsResponse.kind !== MessageFromWorkerKind.Items) {
      throw new Error("unexpected response kind");
    }

    return new Module(constructResponse.moduleId, itemsResponse.result);
  }

  getCacheKey(range: Range): string {
    return `${range.start}-${range.end}`;
  }

  async printRich(range: Range): Promise<PrintPart[]> {
    const cacheKey = this.getCacheKey(range);
    const cached = this.printRichCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    let response = await sendMessage({
      kind: MessageToWorkerKind.PrintRich,
      id: nextMessageId++,
      moduleId: this.id,
      range,
    });
    if (response.kind !== MessageFromWorkerKind.PrintRich) {
      throw new Error("unexpected response kind");
    }

    this.printRichCache.set(cacheKey, response.result);
    return response.result;
  }

  async printPlain(range: Range): Promise<string> {
    let response = await sendMessage({
      kind: MessageToWorkerKind.PrintPlain,
      id: nextMessageId++,
      moduleId: this.id,
      range,
    });
    if (response.kind !== MessageFromWorkerKind.PrintPlain) {
      throw new Error("unexpected response kind");
    }
    return response.result;
  }
}
