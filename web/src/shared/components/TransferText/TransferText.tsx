import byteSize from 'byte-size';
import clsx from 'clsx';
import './style.scss';
import { Icon } from '../../defguard-ui/components/Icon';

type Props = {
  variant?: 'upload' | 'download';
  data: number;
  icon?: boolean;
};

export const TransferText = ({ data, variant, icon }: Props) => {
  const size = byteSize(data, { precision: 1 });

  return (
    <div className={clsx('transfer-text', variant)}>
      {icon && (
        <Icon
          size={18}
          icon="arrow-big"
          rotationDirection={variant === 'download' ? 'right' : 'left'}
        />
      )}
      <span>{`${size.value} ${size.unit}`}</span>
    </div>
  );
};
