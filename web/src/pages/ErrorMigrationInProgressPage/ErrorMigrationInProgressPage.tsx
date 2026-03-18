import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import { FlowEndImageVariant } from '../../shared/components/FlowEndLayout/components/FlowEndImage/types';
import { FlowEndLayout } from '../../shared/components/FlowEndLayout/FlowEndLayout';
import type { ButtonProps } from '../../shared/defguard-ui/components/Button/types';
import { useAuth } from '../../shared/hooks/useAuth';

export const ErrorMigrationInProgressPage = () => {
  const navigate = useNavigate();

  const { mutate: actionMutation, isPending } = useMutation({
    mutationFn: api.auth.logout,
    onSuccess: () => {
      useAuth.getState().reset();
      navigate({ to: '/auth/login', replace: true });
    },
    meta: {
      invalidate: [['me'], ['user']],
    },
  });

  const actionProps = useMemo(
    (): ButtonProps => ({
      text: m.flow_end_migration_auth_user_action(),
      variant: 'primary',
      loading: isPending,
      onClick: () => {
        actionMutation();
      },
    }),
    [isPending, actionMutation],
  );

  return (
    <FlowEndLayout
      image={FlowEndImageVariant.AppError}
      title={m.flow_end_migration_auth_user_title()}
      subtitle={m.flow_end_migration_auth_user_subtitle()}
      actionProps={actionProps}
    />
  );
};
