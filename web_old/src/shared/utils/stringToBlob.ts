export const stringToBlob = (value: string): Blob => {
  const blob = new Blob([value.replace(/^[^\S\r\n]+|[^\S\r\n]+$/gm, '')], {
    type: 'text/plain;charset=utf-8',
  });
  return blob;
};
