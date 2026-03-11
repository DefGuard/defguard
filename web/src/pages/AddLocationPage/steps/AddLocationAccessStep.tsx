import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { SelectionSection } from '../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationAccessStep = () => {
  const [selected, setSelected] = useState<Set<string>>(
    new Set(useAddLocationStore.getState().allowed_groups),
  );

  const { data: groups } = useQuery({
    queryFn: api.group.getGroups,
    queryKey: ['group'],
    select: (resp) => resp.data.groups,
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

  const saveChanges = useCallback((values: Set<string>) => {
    useAddLocationStore.setState({
      allowed_groups: Array.from(values),
    });
  }, []);

  return (
    <WizardCard>
      <SelectionSection
        options={selectionOptions}
        selection={selected}
        onChange={setSelected}
      />
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            saveChanges(selected);
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
              saveChanges(selected);
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
