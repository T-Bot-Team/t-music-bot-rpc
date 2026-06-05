import fs from "fs";
import os from "os";
import { PATHS, IS_QUIET } from "../utils/constants";

/**
 * Logs a message to the console and a log file.
 * @param m The message to log.
 * @param force If true, the message will be logged even in quiet mode.
 */
export const log = (m: string, force: boolean = false): void => {
  if (IS_QUIET && !force) return;
  const msg = `[${new Date().toLocaleTimeString()}] ${m}`;
  console.log(msg);
  try {
    fs.appendFileSync(PATHS.logs, msg + os.EOL);
  } catch (e) {}
};
