import React, { useMemo } from 'react';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ManualConfig } from '../../../../shared/components/svg';
import SvgIconInfoNormal from '../../../../shared/components/svg/IconInfoNormal';
import { deviceBreakpoints } from '../../../../shared/constants';
import NetworkSelectButton from './NetworkSelectButton';

interface Props extends React.HTMLAttributes<HTMLInputElement> {
  onChange: (value: unknown) => void;
  value: 'import' | 'manual';
}

// eslint-disable-next-line react/display-name
const Manual = React.forwardRef<HTMLInputElement, Props>(
  ({ onChange, value, ...props }, ref) => {
    const { LL } = useI18nContext();
    const { breakpoint } = useBreakpoint(deviceBreakpoints);
    const getClassName = useMemo(() => {
      const res = ['regular-network'];
      if (value === 'manual') {
        res.push('active');
      }
      if (value === 'import') {
        res.push('not-active');
      }
      return res.join(' ');
    }, [value]);

    const handleChange = () => {
      onChange('manual');
    };

    return (
      <div className={getClassName}>
        {breakpoint !== 'desktop' && <SvgIconInfoNormal />}
        <h3>{LL.wizard.wizardType.manual.title()}</h3>
        {breakpoint === 'desktop' && (
          <>
            <p>{LL.wizard.wizardType.manual.description()}</p>
            <ManualConfig />
          </>
        )}

        <NetworkSelectButton
          active={typeof value !== 'undefined' && value === 'manual'}
          onClick={() => handleChange()}
        />
        <input
          type="radio"
          ref={ref}
          {...props}
          onChange={onChange}
          checked={value === 'manual'}
        />
      </div>
    );
  }
);

export default Manual;
