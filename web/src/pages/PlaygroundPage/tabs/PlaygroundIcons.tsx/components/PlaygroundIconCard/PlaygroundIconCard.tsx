import { titleCase } from 'text-case';
import {
  Icon,
  type IconKindValue,
} from '../../../../../../shared/defguard-ui/components/Icon';
import { useClipboard } from '../../../../../../shared/defguard-ui/hooks/useClipboard';
import './style.scss';
import { Snackbar } from '../../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';

interface Props {
  icon: IconKindValue;
}

export const PlaygroundIconCard = ({ icon }: Props) => {
  const { writeToClipboard } = useClipboard();
  return (
    <TooltipProvider>
      <TooltipTrigger>
        <div
          className="playground-icon-card"
          onClick={() => {
            writeToClipboard(icon).then(() => {
              Snackbar.default(`Copied ${icon}`);
            });
          }}
        >
          <div className="top">
            <Icon icon={icon} />
          </div>
          <div className="bottom">
            <p>{titleCase(icon.replaceAll('-', ' '))}</p>
          </div>
        </div>
      </TooltipTrigger>
      <TooltipContent>
        <p>{icon}</p>
      </TooltipContent>
    </TooltipProvider>
  );
};
