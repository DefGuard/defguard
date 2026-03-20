import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import { FlowEndImageVariant } from '../../shared/components/FlowEndLayout/components/FlowEndImage/types';
import { FlowEndLayout } from '../../shared/components/FlowEndLayout/FlowEndLayout';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';

export const Error404Page = () => {
  const navigate = useNavigate();

  const actionProps = useMemo(
    (): ButtonProps => ({
      variant: 'primary',
      text: m.flow_end_404_action(),
      onClick: () => {
        navigate({ to: '/', replace: true });
      },
    }),
    [navigate],
  );

  return (
    <FlowEndLayout
      image={FlowEndImageVariant.Error404}
      title={m.flow_end_404_title()}
      subtitle={m.flow_end_404_subtitle()}
      actionProps={actionProps}
    />
  );
};
