import './style.scss';
import { useCallback, useEffect, useRef, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Icon, IconKind } from '../../defguard-ui/components/Icon';
import { useClipboard } from '../../defguard-ui/hooks/useClipboard';
import { Snackbar } from '../../defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../defguard-ui/providers/tooltip/TooltipTrigger';

interface Props {
  value: string;
}

export const CodeSnippet = ({ value }: Props) => {
  const timeoutIdRef = useRef<number | null>(null);
  const [tooltipVisible, setTooltipVisible] = useState(false);

  const { writeToClipboard } = useClipboard();

  const handleCopy = useCallback(() => {
    writeToClipboard(value)
      .then(() => {
        setTooltipVisible(true);
        const timeoutId = setTimeout(() => {
          setTooltipVisible(false);
        }, 1_500);
        timeoutIdRef.current = timeoutId;
      })
      .catch((e) => {
        Snackbar.error('Unable to access clipboard.');
        console.error(e);
      });
  }, [value, writeToClipboard]);

  useEffect(() => {
    return () => {
      if (timeoutIdRef.current !== null) {
        clearTimeout(timeoutIdRef.current);
      }
    };
  }, []);

  return (
    <div className="code-snippet">
      <div className="inner">
        <TooltipProvider open={tooltipVisible}>
          <TooltipTrigger>
            <button className="copy" onClick={handleCopy}>
              <Icon icon={tooltipVisible ? IconKind.CheckCircle : IconKind.Copy} />
            </button>
          </TooltipTrigger>
          <TooltipContent>
            <p>{m.cmp_copy_button_tooltip()}</p>
          </TooltipContent>
        </TooltipProvider>
        <pre>
          <code>{value}</code>
        </pre>
      </div>
    </div>
  );
};
