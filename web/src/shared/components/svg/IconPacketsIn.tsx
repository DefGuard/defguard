import type { SVGProps } from 'react';
const SvgIconPacketsIn = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path d="M0 0h16v16H0z" className="a" />
      </clipPath>
      <style>{'.a{fill:#0c8ce0}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="translate(.997 5)"
    >
      <rect width={2} height={2} className="a" rx={1} transform="translate(2 2)" />
      <rect width={2} height={2} className="a" rx={1} transform="translate(5 2)" />
      <g
        style={{
          fill: '#0c8ce0',
        }}
      >
        <path
          d="M4.234 4H1.766L3 1.944 4.234 4Z"
          style={{
            stroke: 'none',
          }}
          transform="rotate(90 6.5 6.5)"
        />
        <path
          d="M3 .944c.332 0 .663.161.857.485l1.234 2.057A1 1 0 0 1 4.234 5H1.766A1 1 0 0 1 .91 3.486l1.234-2.057A.991.991 0 0 1 3 .944Z"
          style={{
            fill: '#0c8ce0',
            stroke: 'none',
          }}
          transform="rotate(90 6.5 6.5)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconPacketsIn;
