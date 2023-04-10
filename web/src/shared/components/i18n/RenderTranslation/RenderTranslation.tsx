import { Fragment, ReactNode, useMemo } from 'react';

type Props = {
  translation: string;
  components: ReactNode[];
};
/**
 * Renders string and replace every instance of `<React>` with given component in order.
 */
export const RenderTranslation = ({ translation, components }: Props) => {
  const segments = useMemo(() => {
    const res = translation.split('<React>');
    if (res.length === 1) {
      throw Error('Translation is missing "<React>" keyword.');
    }
    return res;
  }, [translation]);
  return (
    <>
      {segments.map((val, index) => (
        <Fragment key={index}>
          {val}
          {components[index] ?? null}
        </Fragment>
      ))}
    </>
  );
};
