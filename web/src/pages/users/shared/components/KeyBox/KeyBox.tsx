import './style.scss';

import { saveAs } from 'file-saver';

import { ActionButton } from '../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { useClipboard } from '../../../../../shared/hooks/useClipboard';

interface Props {
  title: string;
  keyValue?: string;
  collapsible?: boolean;
  disabled?: boolean;
  initiallyOpen?: boolean;
}

export const KeyBox = ({ title, keyValue }: Props) => {
  const { writeToClipboard } = useClipboard();

  const handleCopy = () => {
    if (keyValue) {
      writeToClipboard(keyValue);
    }
  };

  const handleDownload = () => {
    const blob = new Blob([keyValue as string], {
      type: 'text/plain;charset=utf-8',
    });
    saveAs(blob, `${title.replace(' ', '_').toLocaleLowerCase()}.txt`);
  };

  if (!keyValue) return null;

  return (
    <div className="key-box">
      <span className="title">{title}</span>
      <div className="actions">
        <ActionButton variant={ActionButtonVariant.COPY} onClick={handleCopy} />
        <ActionButton variant={ActionButtonVariant.DOWNLOAD} onClick={handleDownload} />
      </div>
    </div>
  );
};
