export type DateInputProps = {
  selected: string;
  label?: string;
  errorMessage?: string;
  onChange: (value: string | null) => void;
};
