export type DateInputProps = {
  selected: string;
  label?: string;
  errorMessage?: string;
  disabled?: boolean;
  onChange: (value: string | null) => void;
};
