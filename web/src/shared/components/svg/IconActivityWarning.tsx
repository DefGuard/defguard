import type { SVGProps } from 'react';
const SvgIconActivityWarning = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="icon-activity-warning_svg__a">
        <path d="M0 0h16v16H0z" className="icon-activity-warning_svg__a" />
      </clipPath>
      <style>{'.icon-activity-warning_svg__a{fill:#cb3f3f}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-activity-warning_svg__a)',
      }}
      transform="translate(-784 -142.59)"
    >
      <circle
        cx={6}
        cy={6}
        r={6}
        className="icon-activity-warning_svg__a"
        transform="translate(786 144.59)"
      />
      <path
        d="M2.426-4.7H.984L.757-9.953h1.9ZM.7-3.061a.9.9 0 0 1 .071-.354.8.8 0 0 1 .2-.283 1 1 0 0 1 .312-.187 1.1 1.1 0 0 1 .4-.069 1.1 1.1 0 0 1 .4.069A1 1 0 0 1 2.4-3.7a.8.8 0 0 1 .2.283.9.9 0 0 1 .071.354.9.9 0 0 1-.071.354.8.8 0 0 1-.2.283 1 1 0 0 1-.312.187 1.1 1.1 0 0 1-.4.069 1.1 1.1 0 0 1-.4-.069 1 1 0 0 1-.312-.187.8.8 0 0 1-.2-.283.9.9 0 0 1-.076-.352"
        style={{
          fill: '#fff',
        }}
        transform="translate(790.296 156.543)"
      />
    </g>
  </svg>
);
export default SvgIconActivityWarning;
