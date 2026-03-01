import { module } from "../component-built/component.js";
import { startWorker } from "./WorkerCore.js";

startWorker(() => Promise.resolve(module));
