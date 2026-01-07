import { useCanGoBack, useRouter } from '@tanstack/react-router';
import './style.scss';
import { Fragment } from 'react/jsx-runtime';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { Icon } from '../../defguard-ui/components/Icon';
import { IconButton } from '../../defguard-ui/components/IconButton/IconButton';
import { ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
import type { BreadcrumbsProps } from './types';

export const Breadcrumbs = ({ links }: BreadcrumbsProps) => {
  const router = useRouter();
  const canGoBack = useCanGoBack();
  return (
    <div className="breadcrumbs">
      <div className="track">
        <div className="back">
          <IconButton
            icon="arrow-big"
            iconRotation="left"
            onClick={() => {
              router.history.back();
            }}
            disabled={!canGoBack}
          />
        </div>
        <Divider orientation="vertical" spacing={ThemeSpacing.Xl} />
        <div className="links">
          {links.map((link, index) => (
            <Fragment key={`link-${index + 1}`}>
              {link}
              {index !== links.length - 1 && (
                <Icon
                  icon="arrow-small"
                  rotationDirection="right"
                  staticColor={ThemeVariable.FgMuted}
                />
              )}
            </Fragment>
          ))}
        </div>
      </div>
    </div>
  );
};
