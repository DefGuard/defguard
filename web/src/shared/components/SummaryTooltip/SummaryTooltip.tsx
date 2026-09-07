import './style.scss';

import type { Placement } from '@floating-ui/react';
import clsx from 'clsx';
import { Fragment, type PropsWithChildren } from 'react';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { TooltipContent } from '../../defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../defguard-ui/providers/tooltip/TooltipTrigger';
import type { SummarySection } from './type';

type Props = {
  sections: SummarySection[];
  className?: string;
  placement?: Placement;
};

export const SummaryTooltip = ({
  sections,
  className,
  children,
  placement = 'right-start',
}: Props & PropsWithChildren) => {
  if (sections.length === 0) return children;

  return (
    <TooltipProvider placement={placement}>
      <TooltipTrigger>{children}</TooltipTrigger>
      <TooltipContent className={clsx('summary-tooltip', className)}>
        {sections.map((section, index) => (
          <Fragment key={section.label}>
            {index > 0 && <Divider />}
            <div className="summary-item">
              <p className="label">{section.label}</p>
              <div className="content">
                {section.lines.map((line) => (
                  <p key={`${section.label}-${line}`}>{line}</p>
                ))}
              </div>
            </div>
          </Fragment>
        ))}
      </TooltipContent>
    </TooltipProvider>
  );
};
