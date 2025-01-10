import Markdown from 'react-markdown';
import rehypeRaw from 'rehype-raw';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';

type Props = {
  content: string;
};

const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [...(defaultSchema.tagNames ?? []), 'div', 'br'], // Allow <div> tags
  attributes: {
    ...defaultSchema.attributes,
    div: ['style'], // Allow `style` attribute on <div>
    a: ['href', 'target'], // Allow `href` and `target` on <a>
  },
};

export const RenderMarkdown = ({ content }: Props) => {
  const parse = (): string => {
    const lines = content.split(/\r?\n/);

    // Trim all lines and handle empty lines
    const processedLines = lines.map((line) => {
      const trimmedLine = line.trim();
      return trimmedLine === '' ? '' : trimmedLine; // Replace empty lines with empty strings
    });

    // Remove the first line if it's empty
    if (processedLines.length > 0 && processedLines[0] === '') {
      processedLines.shift();
    }

    // Remove the last line if it's empty
    if (processedLines.length > 0 && processedLines[processedLines.length - 1] === '') {
      processedLines.pop();
    }

    return processedLines.join('\n');
  };
  return (
    <Markdown rehypePlugins={[rehypeRaw, [rehypeSanitize, sanitizeSchema]]}>
      {parse()}
    </Markdown>
  );
};
