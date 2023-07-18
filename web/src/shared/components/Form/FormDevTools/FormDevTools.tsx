import './style.scss';

import { DevTool } from '@hookform/devtools';
import ReactDOM from 'react-dom';
import { Control } from 'react-hook-form';

interface Props {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  control: Control<any, object>;
}

export const DevTools: React.FC<Props> = ({ control }) => {
  const element = document.querySelector('#root');
  if (!element) return null;
  return ReactDOM.createPortal(
    <div className="dev-tools">
      <DevTool control={control} />
    </div>,
    element,
  );
};
