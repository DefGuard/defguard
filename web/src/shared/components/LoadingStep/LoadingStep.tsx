import clsx from 'clsx';
import './style.scss';
import { useMemo } from 'react';
import { AppText } from '../../defguard-ui/components/AppText/AppText';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { Icon, IconKind } from '../../defguard-ui/components/Icon';
import { LoaderSpinner } from '../../defguard-ui/components/LoaderSpinner/LoaderSpinner';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import type { LoadingStepProps } from './types';

type ComponentVariant = 'loading' | 'success' | 'error' | 'default';

export const LoadingStep = ({
  title,
  children,
  error,
  errorMessage,
  success,
  loading,
}: LoadingStepProps) => {
  const variant = useMemo((): ComponentVariant => {
    if (success) return 'success';
    if (error) return 'error';
    if (loading) return 'loading';
    return 'default';
  }, [error, success, loading]);

  return (
    <div className={clsx('loading-step', `variant-${variant}`)}>
      <div className="main-track">
        <div className="icon-container">
          {variant === 'default' && (
            <Icon
              icon={IconKind.EmptyPoint}
              size={20}
              staticColor={ThemeVariable.FgDisabled}
            />
          )}
          {loading && <LoaderSpinner size={20} variant="primary" />}
          {success && (
            <Icon
              icon={IconKind.CheckCircle}
              staticColor={ThemeVariable.FgSuccess}
              size={20}
            />
          )}
          {error && (
            <Icon
              icon={IconKind.WarningFilled}
              staticColor={ThemeVariable.FgCritical}
              size={20}
            />
          )}
        </div>
        <p className={clsx('title')}>{title}</p>
      </div>
      <div className="content-track">
        <div className="content-bar"></div>
        {isPresent(children) && variant === 'error' && (
          <div className="content">
            <Fold open={variant === 'error'}>
              {isPresent(errorMessage) && (
                <>
                  <AppText color={ThemeVariable.FgCritical} font={TextStyle.TBodySm400}>
                    {errorMessage}
                  </AppText>
                  <SizedBox height={ThemeSpacing.Xl} />
                </>
              )}
              {children}
              <SizedBox height={ThemeSpacing.Xl2} />
            </Fold>
          </div>
        )}
      </div>
    </div>
  );
};
