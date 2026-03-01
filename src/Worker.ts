import { module } from "waside";
import { startWorker } from "./WorkerCore.js";

startWorker(() => Promise.resolve(module));
