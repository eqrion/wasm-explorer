import { startWorker } from "../../src/WorkerCore.js";

// self.name is set to the component JS URL via the Worker constructor { name: componentJsUrl }
const componentUrl: string = self.name;
console.log(`[worker] loading component from: ${componentUrl}`);
startWorker(() =>
  import(/* webpackIgnore: true */ componentUrl).then((m: any) => {
    console.log("[worker] component loaded");
    return m.module;
  }),
);
