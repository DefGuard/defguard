import './style.scss';

type Props = {
  text: string;
  onClick: () => void;
  testId?: string;
};

export const DeviceNameButton = ({ text, onClick, testId }: Props) => {
  return (
    <button
      type="button"
      className="device-name-button"
      data-testid={testId}
      onClick={onClick}
    >
      {text}
    </button>
  );
};
