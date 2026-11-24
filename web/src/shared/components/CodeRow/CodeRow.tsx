import './style.scss';

type Props = {
  values: number[];
  testId?: string;
};

export const CodeRow = ({ values, testId }: Props) => {
  return (
    <div className="code-row" data-testid={testId}>
      {values.map((value, index) => (
        <div className="block" key={index}>
          {value}
        </div>
      ))}
    </div>
  );
};
