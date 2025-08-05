import type { Module as ModuleType, Range, PrintPart, Item } from "../component-built/interfaces/local-module-module.js";

export type ModuleId = number;
export type MessageId = number;

export const LoadedMessageId = 0;
export const FirstMessageId = 1;

export enum MessageFromWorkerKind {
  Loaded = "loaded",
  Exception = "exception",
  Construct = "construct",
  Destroy = "destroy",
  PrintRich = "printRich",
  PrintPlain = "printPlain",
  Items = "items",
};

export type MessageFromWorker =
  | { kind: MessageFromWorkerKind.Loaded, id: MessageId }
  | { kind: MessageFromWorkerKind.Exception, id: MessageId, exception: any }
  | { kind: MessageFromWorkerKind.Construct, id: MessageId, moduleId: ModuleId }
  | { kind: MessageFromWorkerKind.Destroy, id: MessageId }
  | { kind: MessageFromWorkerKind.PrintRich, id: MessageId, result: PrintPart[] }
  | { kind: MessageFromWorkerKind.PrintPlain, id: MessageId, result: string }
  | { kind: MessageFromWorkerKind.Items, id: MessageId, result: Item[] }

export enum MessageToWorkerKind {
  Construct = "construct",
  Destroy = "destroy",
  PrintRich = "printRich",
  PrintPlain = "printPlain",
  Items = "items",
};

export type MessageToWorker =
  | { kind: MessageToWorkerKind.Construct, id: MessageId, source: Uint8Array }
  | { kind: MessageToWorkerKind.Destroy, id: MessageId, moduleId: ModuleId }
  | { kind: MessageToWorkerKind.PrintRich, id: MessageId, moduleId: ModuleId, range: Range }
  | { kind: MessageToWorkerKind.PrintPlain, id: MessageId, moduleId: ModuleId, range: Range }
  | { kind: MessageToWorkerKind.Items, id: MessageId, moduleId: ModuleId }
