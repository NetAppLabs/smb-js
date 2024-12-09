import { SmbDirectoryHandle } from '@netapplabs/smb-js'

let SMB_URL = "smb://guest@127.0.0.1:10445/smbtest/test"
console.log(` -- Starting watch on directory ${SMB_URL}`);
let smbHandle = new SmbDirectoryHandle(SMB_URL);

// watch directory 'test' with callback function provided
smbHandle.watch(async (watchEvent) => {
   console.log("watchEvent returned:", watchEvent)
   let filePath = watchEvent.path;
   let fileAction = watchEvent.action;
   console.log(" -- FilePath being noted:", filePath)
   console.log(" -- File Action being noted:", fileAction)

   if (!filePath.startsWith("eventLog.csv")) {
      try {
         // writing event back to filesystem in eventLog file
         let eventLogFileName = "eventLog.csv";
         let smbHandle2 = new SmbDirectoryHandle(SMB_URL);

         let fh = await smbHandle2.getFileHandle(eventLogFileName, { create: true });
         let fileWriter = await fh.createWritable({ keepExistingData: true });
         const logString = watchEvent.path + "," + fileAction + "\n";
         await fileWriter.write(logString);
         await fileWriter.close();
      } catch (err) {
         console.log("Error writing to log", err);
      }
   }
})
