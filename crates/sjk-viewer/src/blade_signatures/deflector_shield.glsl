// Deflector Shield: a starship's deflector field wrapped round the blade, from the hilt to the tip.
// A shell of hexagonal shield cells shimmers round it, brighter at its curved edges, each cell
// slowly flickering; every second or so a hit lands somewhere on it: the cells there flash and a
// ripple races out across the field. A swing brightens it; a clash flares the whole shell.
// @part Hex field
// @part Impact flashes
// @room 8
// @palette {"corona.rim_cool":[0.0,0.25,0.45],"corona.rim_hot":[0.25,0.85,1.0],"corona.inner":[0.75,0.97,1.0],"core.white":[0.92,1.0,1.0],"core.fringe_cool":[0.2,0.8,1.0],"core.fringe_hot":[0.75,0.97,1.0],"light.color":[0.25,0.85,1.0],"trail":[0.25,0.85,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.4, 0.9, 1.0);
  // round 17 (Creyon: the blade's tip poked out through the dome): the dome closes over the tip
  float R = 2.6, yb = 0.5, yt = L + 1.2;
  if (abs(rd.z) > 0.0001){
    vec3 p = ro + rd*(-ro.z/rd.z);
    // the shell's radius at this height: a cylinder from the hilt with a round dome at the tip
    float rr = R*sqrt(clamp(1.0 - pow(max(p.y - (yt - R), 0.0)/R, 2.0), 0.0, 1.0));
    if (abs(p.x) < rr + 0.6 && p.y > yb - 0.5 && p.y < yt + 0.5){
      float xr = clamp(p.x/max(rr, 0.01), -1.0, 1.0);
      float inside = 1.0 - smoothstep(0.96, 1.04, abs(p.x)/max(rr, 0.01));
      // brighter at its curved edges, as a shell seen side on
      float shell = 0.35 + 0.65*pow(1.0 - sqrt(max(1.0 - xr*xr, 0.0)), 2.0);
      // hexagonal cells over the unrolled shell
      vec2 u = vec2(asin(xr)*R*0.9, p.y*0.5);
      vec2 sA = vec2(1.0, 1.732);
      vec2 cA = floor(u/sA) + 0.5, cB = floor((u - vec2(0.5, 0.866))/sA) + 0.5;
      vec2 qA = u - cA*sA, qB = u - (cB*sA + vec2(0.5, 0.866));
      vec2 q = dot(qA, qA) < dot(qB, qB) ? qA : qB;
      vec2 id = dot(qA, qA) < dot(qB, qB) ? cA : cB + vec2(0.37, 0.71);
      vec2 aq = abs(q);
      float hx = max(aq.x*0.5 + aq.y*0.866, aq.x);
      float edge = halo(0.5 - hx, 0.035);
      float flick = 0.5 + 0.5*sin(t*1.3 + h21(id)*6.2832);
      float lit = max(cl, 0.35*sw);
      float base = 1.0 - smoothstep(yt - 0.6, yt + 0.4, p.y + (R - rr)*0.0);
      float foot = smoothstep(yb - 0.5, yb + 0.8, p.y);
      fx += c*(edge*(0.3 + 0.35*flick) + 0.08*shell)*inside*foot*(1.0 + 1.6*lit)*part(0.0);
      fx += c*halo(abs(abs(p.x) - rr), 0.12)*0.5*foot*(1.0 + 1.5*lit)*part(0.0);
      // hits: every second or so a bolt lands, the cells there flash and a ripple races out
      for (int i = 0; i < 2; i++){
        float fi = float(i);
        float cyc = t*0.8 + fi*0.5;
        float k = floor(cyc), qq = fract(cyc);
        vec2 at = vec2((h11(k*3.7 + fi) - 0.5)*R*1.1, mix(y0, y1 - 2.0, h11(k*5.3 + fi*2.0))*0.5);
        float dist = length(u - at);
        float flash = exp(-qq*5.0);
        float ripple = halo(abs(dist - qq*4.0), 0.3)*(1.0 - qq);
        fx += (vec3(1.0)*halo(dist, 0.6)*flash*2.0 + c*(edge*halo(dist, 1.8)*flash*3.0 + ripple*(0.5 + edge)*1.4))*inside*foot*part(1.0);
      }
    }
  }
  return fx*uFx;
}
