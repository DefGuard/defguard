import saveAs from 'file-saver';

export const downloadWGConfig = (config: string, fileName: string) => {
  const blob = new Blob([config.replace(/^[^\S\r\n]+|[^\S\r\n]+$/gm, '')], {
    type: 'text/plain;charset=utf-8',
  });
  saveAs(blob, `${fileName.toLowerCase()}.conf`);
};
