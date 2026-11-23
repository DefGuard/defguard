import { m } from '../../../paraglide/messages';
import { ActionTooltip } from '../../defguard-ui/components/ActionTooltip/ActionTooltip';
import { Button } from '../../defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../defguard-ui/components/Button/types';

type Props = Omit<ButtonProps, 'onClick' | 'ref'> & {
  onDownload: () => void;
};

export const DownloadButton = ({
  onDownload,
  variant = 'outlined',
  iconLeft = 'download',
  ...props
}: Props) => {
  return (
    <ActionTooltip text={m.cmp_download_button_tooltip()}>
      {({ show }) => (
        <Button
          {...props}
          variant={variant}
          iconLeft={iconLeft}
          onClick={() => {
            onDownload();
            show();
          }}
        />
      )}
    </ActionTooltip>
  );
};
