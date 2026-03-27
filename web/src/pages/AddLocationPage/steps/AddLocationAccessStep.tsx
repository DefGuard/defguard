import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { SelectionSection } from '../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../../shared/defguard-ui/components/Toggle/Toggle';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationAccessStep = () => {
  const [allowAllGroups, setAllowAllGroups] = useState(
    useAddLocationStore.getState().allow_all_groups,
  );
  const [selected, setSelected] = useState<Set<string>>(
    new Set(useAddLocationStore.getState().allowed_groups),
  );

  const { data: groups } = useQuery({
    queryFn: api.group.getGroups,
    queryKey: ['group'],
  });

  const selectionOptions = useMemo(() => {
    if (!groups) return [];
    return groups.map(
      (group): SelectionOption<string> => ({
        id: group,
        label: group,
      }),
    );
  }, [groups]);

  const saveChanges = useCallback((values: Set<string>, allowAll: boolean) => {
    useAddLocationStore.setState({
      allow_all_groups: allowAll,
      allowed_groups: Array.from(values),
    });
  }, []);

  return (
    <WizardCard>
      <Toggle
        label={m.location_access_all_groups_have_access()}
        active={allowAllGroups}
        onClick={() => {
          const value = !allowAllGroups;
          setAllowAllGroups(value);
        }}
      />
      {!allowAllGroups && (
        <>
          <SizedBox height={ThemeSpacing.Xl} />
          <SelectionSection
            options={selectionOptions}
            selection={selected}
            onChange={setSelected}
          />
        </>
      )}
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            saveChanges(selected, allowAllGroups);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.Mfa,
            });
          }}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            testId="acl-continue"
            onClick={() => {
              saveChanges(selected, allowAllGroups);
              useAddLocationStore.setState({
                activeStep: AddLocationPageStep.Firewall,
              });
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
