import './style.scss';

import { useEffect, useMemo, useState } from 'react';

import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../defguard-ui/components/AppText/AppText';
import { Badge } from '../../../defguard-ui/components/Badge/Badge';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { Icon } from '../../../defguard-ui/components/Icon';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import { RenderMarkdown } from '../../../defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../defguard-ui/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../hooks/modalControls/modalTypes';
import type { OpenAppUpdateModal } from '../../../hooks/modalControls/types';
import { Controls } from '../../Controls/Controls';
import updateImage from './update-image.png';

const modalNameKey = ModalName.AppUpdate;
const DISMISSED_UPDATE_KEY = 'dismissed-update-version';

type ModalData = OpenAppUpdateModal;

const transformGithubNotes = (notes: string): string =>
  notes.replace(
    /^(\* |- )(.*?) by (@\S+) in (https?:\/\/\S+)$/gm,
    (_, bullet, title, user, url) => `${bullet}[${title}](${url}) by ${user}`,
  );

export const AppUpdateModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <ModalFoundation
      id="app-update-modal"
      contentClassName="app-update-modal"
      isOpen={isOpen}
      afterClose={() => {
        setModalData(null);
      }}
    >
      <div className="tracks">
        <div className="content-track">
          {isPresent(modalData) && <ModalContent data={modalData} />}
        </div>
        <div className="media-track">
          <img src={updateImage} alt="defguard update" />
        </div>
      </div>
    </ModalFoundation>
  );
};

const ModalContent = ({ data }: { data: ModalData }) => {
  const handleDismiss = () => {
    localStorage.setItem(DISMISSED_UPDATE_KEY, data.version);
    closeModal(modalNameKey);
  };

  const { subtitle, body } = useMemo(() => {
    const trimmed = data.notes.trim();
    const firstBlank = trimmed.search(/\n\s*\n/);
    const rawSubtitle = firstBlank === -1 ? trimmed : trimmed.slice(0, firstBlank).trim();
    const cleanSubtitle = rawSubtitle.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
    return {
      subtitle: cleanSubtitle,
      body: transformGithubNotes(
        (firstBlank === -1 ? '' : trimmed.slice(firstBlank).trim())
          .replace(/^\*\*Full Changelog\*\*:.*$/gm, '')
          .replace(/^(#{1,6}) (.+)$/gm, (_, hashes, title) => {
            const normalized =
              title.charAt(0).toUpperCase() + title.slice(1).toLowerCase();
            const withColon = normalized.endsWith(':') ? normalized : `${normalized}:`;
            return `${hashes} ${withColon}`;
          })
          .trimEnd(),
      ),
    };
  }, [data.notes]);

  return (
    <>
      <div className="header-section">
        {data.critical && (
          <Badge
            text={m.modal_app_update_critical_badge()}
            variant="critical"
            showIcon
            icon="status-important"
          />
        )}
        <AppText font={TextStyle.TTitleH1} color={ThemeVariable.FgDefault}>
          {m.modal_app_update_title()}
        </AppText>
        <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
          {subtitle}
        </AppText>
      </div>
      <Divider spacing={ThemeSpacing.Lg} />
      <RenderMarkdown content={body} />
      <Divider spacing={ThemeSpacing.Lg} />
      <a
        className="changelog-link"
        href={data.release_notes_url}
        target="_blank"
        rel="noopener noreferrer"
      >
        <Icon icon="arrow-big" size={20} />
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgAction}>
          {m.modal_app_update_full_changelog()}
        </AppText>
      </a>
      <Controls>
        <a href={data.update_url} target="_blank" rel="noopener noreferrer">
          <Button
            text={m.modal_app_update_go_to_release()}
            variant="primary"
            iconRight="open-in-new-window"
          />
        </a>
        <Button
          text={m.modal_app_update_dismiss()}
          variant="secondary"
          onClick={handleDismiss}
        />
      </Controls>
    </>
  );
};
