import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { LoginMainPage } from '../../pages/auth/LoginMain/LoginMainPage';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAuth } from '../../shared/hooks/useAuth';

const searchSchema = z.object({
  authError: z.string().optional(),
  redirect: z.literal('/add-location').optional(),
});

export const Route = createFileRoute('/auth/login')({
  validateSearch: searchSchema,
  beforeLoad: ({ search }) => {
    if (isPresent(search.redirect)) {
      useAuth.setState({ redirectAfterLogin: search.redirect });
    }
  },
  component: LoginMainPage,
});
