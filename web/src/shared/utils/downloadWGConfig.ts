import saveAs from 'file-saver';

export const downloadWGConfig = (config: string, fileName: string) => {
  const blob = new Blob([config.replace(/^[^\S\r\n]+|[^\S\r\n]+$/gm, '')], {
    // octet-stream is used here as a workaround: some browsers will append
    // an additional .txt extension to the file name if the MIME type is text/plain.
    type: 'application/octet-stream',
  });
  saveAs(blob, `${fileName.toLowerCase()}.conf`);
};
