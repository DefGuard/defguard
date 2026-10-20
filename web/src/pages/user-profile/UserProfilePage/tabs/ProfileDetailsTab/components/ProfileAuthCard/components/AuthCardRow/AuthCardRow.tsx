import './style.scss';
import { type PropsWithChildren, useState } from 'react';
import { Badge } from '../../../../../../../../../shared/defguard-ui/components/Badge/Badge';
import type { BadgeProps } from '../../../../../../../../../shared/defguard-ui/components/Badge/types';
import { Icon } from '../../../../../../../../../shared/defguard-ui/components/Icon';
import type { IconKindValue } from '../../../../../../../../../shared/defguard-ui/components/Icon/icon-types';
import { IconButtonMenu } from '../../../../../../../../../shared/defguard-ui/components/IconButtonMenu/IconButtonMenu';
import type { MenuProps } from '../../../../../../../../../shared/defguard-ui/components/Menu/types';
import { TooltipContent } from '../../../../../../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { isPresent } from '../../../../../../../../../shared/defguard-ui/utils/isPresent';

const titleTooltipDelayMs = 500;

type Props = PropsWithChildren & {
  title: string;
  icon: IconKindValue;
  menu?: MenuProps;
  badge?: BadgeProps;
  testId?: string;
};

export const AuthCardRow = ({ title, icon, menu, badge, testId, children }: Props) => {
  const [isTitleTruncated, setTitleTruncated] = useState(false);

  return (
    <div className="auth-card-row" data-testid={testId}>
      <Icon icon={icon} />
      <div className="content">
        <div className="top">
          <TooltipProvider openDelayMs={titleTooltipDelayMs}>
            <TooltipTrigger>
              <p
                onPointerEnter={(event) => {
                  const element = event.currentTarget;
                  setTitleTruncated(element.scrollWidth > element.clientWidth);
                }}
              >
                {title}
              </p>
            </TooltipTrigger>
            {isTitleTruncated && (
              <TooltipContent>
                <p>{title}</p>
              </TooltipContent>
            )}
          </TooltipProvider>
          {isPresent(badge) && <Badge {...badge} />}
        </div>
        {isPresent(children) && <div className="bottom">{children}</div>}
      </div>
      <div className="controls">
        {isPresent(menu) && menu.itemGroups.length > 0 && (
          <IconButtonMenu icon="menu" menuItems={menu.itemGroups} />
        )}
      </div>
    </div>
  );
};
