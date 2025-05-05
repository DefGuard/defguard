export type DateInputProps = {
  selected: string;
  label?: string;
  errorMessage?: string;
  disabled?: boolean;
  showTimeSelection?: boolean;
  onChange: (value: string | null) => void;
};
