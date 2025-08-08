import type {
  Module as ModuleType,
  Range,
  PrintPart,
  Item,
  ValidateError,
} from "../component-built/interfaces/local-module-module.js";

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
}

export type MessageFromWorker =
  | { kind: MessageFromWorkerKind.Loaded; id: MessageId }
  | { kind: MessageFromWorkerKind.Exception; id: MessageId; exception: any }
  | {
      kind: MessageFromWorkerKind.Construct;
      id: MessageId;
      moduleId: ModuleId;
      items: Item[];
      validateError: ValidateError | null;
    }
  | { kind: MessageFromWorkerKind.Destroy; id: MessageId }
  | {
      kind: MessageFromWorkerKind.PrintRich;
      id: MessageId;
      result: PrintPart[];
    }
  | { kind: MessageFromWorkerKind.PrintPlain; id: MessageId; result: string };

export enum MessageToWorkerKind {
  Construct = "construct",
  Destroy = "destroy",
  PrintRich = "printRich",
  PrintPlain = "printPlain",
}

export type MessageToWorker =
  | { kind: MessageToWorkerKind.Construct; id: MessageId; source: Uint8Array }
  | { kind: MessageToWorkerKind.Destroy; id: MessageId; moduleId: ModuleId }
  | {
      kind: MessageToWorkerKind.PrintRich;
      id: MessageId;
      moduleId: ModuleId;
      range: Range;
    }
  | {
      kind: MessageToWorkerKind.PrintPlain;
      id: MessageId;
      moduleId: ModuleId;
      range: Range;
    };
