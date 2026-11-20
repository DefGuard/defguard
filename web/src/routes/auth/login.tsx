import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { LoginMainPage } from '../../pages/auth/LoginMain/LoginMainPage';
import { useAuth } from '../../shared/hooks/useAuth';
import { sanitizeRedirectUrl } from '../../shared/utils/sanitizeRedirectUrl';

const searchSchema = z.object({
  authError: z.string().optional(),
  authErrorDetail: z.string().optional(),
  redirect: z.literal('/add-location').optional(),
  r: z.string().optional().catch(undefined),
});

export const Route = createFileRoute('/auth/login')({
  validateSearch: searchSchema,
  beforeLoad: ({ search }) => {
    useAuth.setState({
      redirectAfterLogin: search.redirect,
      forwardAuthRedirect: sanitizeRedirectUrl(search.r),
    });
  },
  component: LoginMainPage,
});
