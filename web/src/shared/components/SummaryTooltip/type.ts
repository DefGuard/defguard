export type SummaryLine = {
  text: string;
  warning?: boolean;
};

export type SummarySection = {
  label: string;
  lines: SummaryLine[];
};
