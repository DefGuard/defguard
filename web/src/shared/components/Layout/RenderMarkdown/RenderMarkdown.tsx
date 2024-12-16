import Markdown from 'react-markdown';

type Props = {
  content: string;
};
export const RenderMarkdown = ({ content }: Props) => {
  const parse = (): string => {
    const lines = content.split(/\r?\n/);

    const processedLines: string[] = [];

    for (let i = 0; i < lines.length; i++) {
      const isLastLine = i === lines.length - 1;
      const currentLine = lines[i];

      if (isLastLine && currentLine.trim() === '') {
        processedLines.push(currentLine);
      } else {
        processedLines.push(currentLine.trim());
      }
    }

    return processedLines.join('\n');
  };
  return <Markdown>{parse()}</Markdown>;
};
