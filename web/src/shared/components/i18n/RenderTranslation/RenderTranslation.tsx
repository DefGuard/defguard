import { ReactNode } from 'react';
import { LocalizedString } from 'typesafe-i18n';

type Props = {
  translation: LocalizedString;
  component: ReactNode;
};

export const RenderTranslation = ({ translation, component }: Props) => {
  if (!translation.includes('<React>')) {
    throw Error('Given translation does not contain component keyword');
  }

  const [prefix, postfix] = translation.split('<React>') as LocalizedString[];

  return (
    <>
      {prefix}
      {component}
      {postfix}
    </>
  );
};
