import { check } from "@tauri-apps/plugin-updater";
import { ask } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";

export async function checkForUpdates() {
  const update = await check();
  if (update) {
    if (
      !(await ask(
        `A new version is available: ${update.version}. Do you want to update?`
      ))
    )
      return;

    let downloaded = 0;
    let contentLength: number | undefined = 0;

    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength = event.data.contentLength;
          console.log(`started downloading ${event.data.contentLength} bytes`);
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          console.log(`downloaded ${downloaded} from ${contentLength}`);
          break;
        case "Finished":
          console.log("download finished");
          break;
      }
    });

    await relaunch();
  }
}
