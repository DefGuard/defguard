import { externalLink } from '../constants';

export const openClientLink = (value?: string): void => {
  const href = value ?? externalLink.defguard.download;
  openVirtualLink(href);
};

export const openVirtualLink = (value: string): void => {
  const anchorElement = document.createElement('a');
  anchorElement.style.display = 'none';
  anchorElement.href = value;
  anchorElement.target = '_blank';
  anchorElement.rel = 'noopener noreferrer';
  document.body.appendChild(anchorElement);
  anchorElement.click();
  anchorElement.remove();
};
