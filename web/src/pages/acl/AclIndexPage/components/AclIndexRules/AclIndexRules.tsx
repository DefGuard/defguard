import './style.scss';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router';

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
      rules: { getRules, deleteRule },
    },
  } = useApi();
  const queryClient = useQueryClient();

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
              navigate('/admin/acl/form');
            }}
          />
        </div>
      </header>
      {Array.isArray(aclRules) && (
        <ul>
          {aclRules.map((rule) => (
            <li key={rule.id}>
              <span>{rule.name}</span>
              <button
                onClick={() => {
                  navigate(`/admin/acl/form?edit=1&rule=${rule.id}`);
                }}
                type="button"
              >
                Edit
              </button>
              <button
                onClick={() => {
                  void deleteRule(rule.id).then(() => {
                    void queryClient.invalidateQueries({
                      queryKey: [QueryKeys.FETCH_ACL_RULES],
                    });
                  });
                }}
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};
