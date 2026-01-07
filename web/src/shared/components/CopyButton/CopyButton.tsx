import { useEffect, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Button } from '../../defguard-ui/components/Button/Button';
import { useClipboard } from '../../defguard-ui/hooks/useClipboard';
import { TooltipContent } from '../../defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../defguard-ui/providers/tooltip/TooltipTrigger';

type Props = {
  value: string;
};

export const CopyButton = ({ value }: Props) => {
  const [open, setOpen] = useState(false);
  const { writeToClipboard } = useClipboard();

  useEffect(() => {
    if (open) {
      setTimeout(() => {
        setOpen(false);
      }, 1000);
    }
  }, [open]);

  return (
    <TooltipProvider open={open} onOpenChange={setOpen}>
      <TooltipTrigger>
        <Button
          variant="outlined"
          text={m.cmp_copy_button()}
          onClick={() => {
            writeToClipboard(value);
            setOpen(true);
          }}
        />
      </TooltipTrigger>
      <TooltipContent>
        <p>{m.cmp_copy_button_tooltip()}</p>
      </TooltipContent>
    </TooltipProvider>
  );
};
