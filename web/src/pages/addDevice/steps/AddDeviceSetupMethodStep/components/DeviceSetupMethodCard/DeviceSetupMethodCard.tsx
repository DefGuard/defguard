import './style.scss';

import { isUndefined } from 'lodash-es';
import { ReactNode } from 'react';

import SvgIconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import SvgIconOutsideLink from '../../../../../../shared/defguard-ui/components/svg/IconOutsideLink';

type Props = {
  title: string;
  subtitle: string;
  logo: ReactNode;
  selected: boolean;
  link?: string;
  linkText?: string;
  onSelect: () => void;
};
export const DeviceSetupMethodCard = ({
  title,
  link,
  linkText,
  selected,
  logo,
  subtitle,
  onSelect,
}: Props) => {
  return (
    <div className="device-setup-method">
      <h3>{title}</h3>
      <p className="sub-title">{subtitle}</p>
      {logo && <div className="logo-wrapper">{logo}</div>}
      <Button
        size={ButtonSize.LARGE}
        text={!selected ? 'Select' : undefined}
        icon={selected ? <SvgIconCheckmarkWhite /> : undefined}
        styleVariant={selected ? ButtonStyleVariant.SAVE : ButtonStyleVariant.PRIMARY}
        onClick={onSelect}
      />
      {!isUndefined(link) && !isUndefined(linkText) && (
        <a href={link} target="_blank" rel="noopener noreferrer">
          <span>{linkText}</span>
          <SvgIconOutsideLink />
        </a>
      )}
    </div>
  );
};
