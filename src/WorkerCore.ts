import type {
  Module as ModuleType,
} from "../component-built/interfaces/local-module-module.js";
import type {
  ModuleId,
  MessageToWorker,
  MessageFromWorker,
} from "./Messages.js";
import {
  LoadedMessageId,
  MessageToWorkerKind,
  MessageFromWorkerKind,
} from "./Messages.js";

// workaround for https://github.com/Microsoft/TypeScript/issues/20595
declare function postMessage(
  message: MessageFromWorker,
  transfer?: Transferable[],
): void;

export async function startWorker(
  loadModule: () => Promise<{ Module: new (bytes: Uint8Array) => ModuleType }>,
) {
  let { Module } = await loadModule();

  postMessage({ kind: MessageFromWorkerKind.Loaded, id: LoadedMessageId });

  let nextModuleId: ModuleId = 0;
  let modules: { [id: ModuleId]: ModuleType } = {};

  addEventListener("message", ({ data }: { data: MessageToWorker }) => {
    try {
      switch (data.kind) {
        case MessageToWorkerKind.Construct: {
          let moduleId = nextModuleId++;
          let result = new Module(new Uint8Array(data.source));
          modules[moduleId] = result;
          let validateError = result.validate();
          postMessage({
            kind: MessageFromWorkerKind.Construct,
            id: data.id,
            moduleId,
            validateError: validateError ?? null,
            items: validateError ? [] : result.items(),
          });
          return;
        }
        case MessageToWorkerKind.Destroy: {
          delete modules[data.moduleId];
          postMessage({ kind: MessageFromWorkerKind.Destroy, id: data.id });
          return;
        }
        case MessageToWorkerKind.GetSource: {
          let module = modules[data.moduleId];
          let source = module.source();
          postMessage(
            {
              kind: MessageFromWorkerKind.GetSource,
              id: data.id,
              source,
            },
            [source.buffer],
          );
          return;
        }
        case MessageToWorkerKind.PrintRich: {
          let module = modules[data.moduleId];
          let result = module.printRich(data.definitionId);
          postMessage({
            kind: MessageFromWorkerKind.PrintRich,
            id: data.id,
            result,
          });
          return;
        }
        case MessageToWorkerKind.PrintPlain: {
          let module = modules[data.moduleId];
          let result = module.printPlain(data.definitionId);
          postMessage({
            kind: MessageFromWorkerKind.PrintPlain,
            id: data.id,
            result,
          });
          return;
        }
        default: {
          // @ts-expect-error the above should be exhaustive
          type remaining = typeof data.kind;
          console.error("got unknown message", data);
        }
      }
    } catch (err) {
      postMessage({
        kind: MessageFromWorkerKind.Exception,
        id: data.id,
        exception: err,
      });
    }
  });
}
