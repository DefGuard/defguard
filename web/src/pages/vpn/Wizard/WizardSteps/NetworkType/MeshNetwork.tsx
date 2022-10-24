import React, { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import useBreakpoint from 'use-breakpoint';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import SvgIconInfoNormal from '../../../../../shared/components/svg/IconInfoNormal';
import SvgImageMeshNetwork from '../../../../../shared/components/svg/ImageMeshNetwork';
import { deviceBreakpoints } from '../../../../../shared/constants';

interface Props extends React.HTMLAttributes<HTMLInputElement> {
  onChange: (value: unknown) => void;
  value: 'mesh' | 'regular';
}

// eslint-disable-next-line react/display-name
const MeshNetwork = React.forwardRef<HTMLInputElement, Props>(
  ({ onChange, value, ...props }, ref) => {
    const { t } = useTranslation('en');
    const { breakpoint } = useBreakpoint(deviceBreakpoints);

    const getClassName = useMemo(() => {
      const res = ['mesh-network'];
      if (value === 'mesh') {
        res.push('active');
      }
      if (value === 'regular') {
        res.push('not-active');
      }
      return res.join(' ');
    }, [value]);

    return (
      <div className={getClassName}>
        {breakpoint !== 'desktop' && <SvgIconInfoNormal />}
        <h3>{t('wizard.networkType.meshNetwork.title')}</h3>
        {breakpoint === 'desktop' && (
          <p>{t('wizard.networkType.meshNetwork.description')}</p>
        )}
        {breakpoint === 'desktop' && <SvgImageMeshNetwork />}
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.BIG}
          text={t('wizard.common.select')}
          disabled={true}
        />
        <input
          type="radio"
          ref={ref}
          {...props}
          onChange={onChange}
          checked={value === 'mesh'}
        />
      </div>
    );
  }
);

export default MeshNetwork;
