import './style.scss';
import { m } from '../../../paraglide/messages';
import { ActionTooltip } from '../../defguard-ui/components/ActionTooltip/ActionTooltip';
import { Icon, IconKind } from '../../defguard-ui/components/Icon';
import { useClipboard } from '../../defguard-ui/hooks/useClipboard';
import { Snackbar } from '../../defguard-ui/providers/snackbar/snackbar';

interface Props {
  value: string;
}

export const CodeSnippet = ({ value }: Props) => {
  const { writeToClipboard } = useClipboard();

  return (
    <div className="code-snippet">
      <div className="inner">
        <ActionTooltip text={m.cmp_copy_button_tooltip()}>
          {({ show, isOpen }) => (
            <button
              className="copy"
              onClick={() => {
                void writeToClipboard(value).then((copied) => {
                  if (copied) {
                    show();
                  } else {
                    Snackbar.error(m.cmp_copy_button_error());
                  }
                });
              }}
            >
              <Icon icon={isOpen ? IconKind.CheckCircle : IconKind.Copy} />
            </button>
          )}
        </ActionTooltip>
        <pre>
          <code>{value}</code>
        </pre>
      </div>
    </div>
  );
};
