import { m } from '../../../paraglide/messages';
import { ActionTooltip } from '../../defguard-ui/components/ActionTooltip/ActionTooltip';
import { Button } from '../../defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../defguard-ui/components/Button/types';
import { useClipboard } from '../../defguard-ui/hooks/useClipboard';

type Props = Omit<ButtonProps, 'onClick' | 'ref' | 'text'> & {
  value: string;
  text?: string;
};

export const CopyButton = ({
  value,
  text = m.cmp_copy_button(),
  variant = 'outlined',
  iconLeft = 'copy',
  ...props
}: Props) => {
  const { writeToClipboard } = useClipboard();

  return (
    <ActionTooltip text={m.cmp_copy_button_tooltip()}>
      {({ show }) => (
        <Button
          {...props}
          text={text}
          variant={variant}
          iconLeft={iconLeft}
          onClick={() => {
            void writeToClipboard(value);
            show();
          }}
        />
      )}
    </ActionTooltip>
  );
};
