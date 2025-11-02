export const downloadText = (content: string, filename: string, extension?: 'txt') => {
  extension = extension ?? 'txt';
  const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
  downloadFile(blob, filename, extension);
};

export const downloadFile = (blob: Blob, filename: string, extension: string) => {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.style = 'visibility: hidden;';
  a.download = `${filename}.${extension}`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);

  // workaround for firefox
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 5_000);
};
