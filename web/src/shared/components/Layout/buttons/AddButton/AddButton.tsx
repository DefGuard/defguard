import './style.scss';

import clsx from 'clsx';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Button } from '../../../../defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../defguard-ui/components/Layout/Button/types';
import SvgIconPlusWhite from '../../../svg/IconPlusWhite';

type Props = {
  onClick?: () => void;
  loading?: boolean;
  text?: string;
  className?: string;
};

export const AddButton = ({ loading, onClick, text, className }: Props) => {
  const { LL } = useI18nContext();
  const defaultText = LL.common.controls.addNew();

  return (
    <Button
      text={text ?? defaultText}
      className={clsx('add-btn', className)}
      onClick={onClick}
      loading={loading}
      size={ButtonSize.SMALL}
      styleVariant={ButtonStyleVariant.PRIMARY}
      icon={<SvgIconPlusWhite />}
    />
  );
};
