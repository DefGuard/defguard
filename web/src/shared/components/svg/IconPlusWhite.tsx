import type { SVGProps } from 'react';
const SvgIconPlusWhite = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#617684',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#617684}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
    >
      <rect width={10} height={2} className="c" rx={1} transform="rotate(-90 13 3)" />
      <rect width={10} height={2} className="c" rx={1} transform="rotate(-180 8 6)" />
    </g>
  </svg>
);
export default SvgIconPlusWhite;
