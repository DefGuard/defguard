import './style.scss';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';

type Props = {
  title: string;
  text: string;
};
export const FormSection = ({ title, text }: Props) => {
  return (
    <div className="form-section">
      <p className="title">{title}</p>
      <SizedBox height={ThemeSpacing.Xs} />
      <p className="content">{text}</p>
    </div>
  );
};
