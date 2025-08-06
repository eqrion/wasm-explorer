import * as React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app.js";

createRoot(document.getElementById("root") as Element).render(<App/>);
