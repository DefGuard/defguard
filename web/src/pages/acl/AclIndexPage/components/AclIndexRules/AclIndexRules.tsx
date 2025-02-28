import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router';
import { Link } from 'react-router-dom';

import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';

export const AclIndexRules = () => {
  const navigate = useNavigate();
  const {
    acl: {
      rules: { getRules },
    },
  } = useApi();
  const { data: aclRules } = useQuery({
    queryFn: getRules,
    queryKey: [QueryKeys.FETCH_ACL_RULES],
    refetchOnMount: true,
  });

  return (
    <div id="acl-rules">
      <header>
        <h2>Rules</h2>
        <div className="controls">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text="Add new"
            onClick={() => {
              navigate('/admin/acl/create');
            }}
          />
        </div>
      </header>
      {aclRules && (
        <ul>
          {aclRules.map((rule) => (
            <li key={rule.id}>
              <Link to={`/admin/acl/edit?rule=${rule.id}`}>{rule.name}</Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
