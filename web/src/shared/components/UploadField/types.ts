export interface UploadFieldProps {
  value?: File | null;
  className?: string;
  id?: string;
  error?: string;
  loading?: boolean;
  disabled?: boolean;
  acceptedExtensions?: string[];
  testId?: string;
  title?: string;
  onChange?: (value: File | null) => void;
}
