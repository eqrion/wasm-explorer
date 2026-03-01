import { startWorker } from "../../src/WorkerCore.js";

// self.name is set to the component JS URL via the Worker constructor { name: componentJsUrl }
const componentUrl: string = self.name;
startWorker(() =>
  import(/* webpackIgnore: true */ componentUrl).then((m: any) => m.module),
);
