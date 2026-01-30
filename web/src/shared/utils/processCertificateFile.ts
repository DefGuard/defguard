import { Snackbar } from '../defguard-ui/providers/snackbar/snackbar';
export const processCertificateFile = async (
  file: File | null,
): Promise<string | null> => {
  if (!file) return null;
  try {
    const content = await file.text();
    return content.replace(/\r\n/g, '\n').trim();
  } catch (error) {
    Snackbar.error('Failed to read certificate file');
    console.error('Failed to read certificate file:', error);
    return null;
  }
};
