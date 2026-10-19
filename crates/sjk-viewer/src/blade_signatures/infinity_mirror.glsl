// Infinity Mirror: a magenta-and-cyan LED blade standing between two mirrors. Its reflections repeat
// out to both sides forever, each copy smaller, dimmer and shifted in colour, converging toward a
// vanishing point, and the whole endless row slides outward so new reflections keep appearing at the
// blade. Mirror frames peel off and recede, LEDs chasing round them, and mirror shards drift away.
// A swing sends the reflections racing; a clash flashes one huge frame and a spray of shards.
// @room 8
// @palette {"corona.rim_cool":[0.55,0.12,0.60],"corona.rim_hot":[1.0,0.45,0.95],"corona.inner":[1.0,0.9,1.0],"core.white":[1.0,0.97,1.0],"core.fringe_cool":[0.40,0.70,1.0],"core.fringe_hot":[1.0,0.55,0.95],"light.color":[0.90,0.35,0.95],"trail":[0.90,0.40,0.95]}

vec3 im_blade(vec2 b, float px, float k, float scale){
  float L = uLength*scale;
  float yc = clamp(b.y, 0.0, L - 0.35*scale);
  float d = length(vec2(b.x, b.y - yc))/scale;
  vec3 mag = vec3(1.0, 0.3, 0.85), cyn = vec3(0.3, 0.9, 1.0);
  // LED segments chasing up the blade
  float seg = fract(b.y/scale*0.9 - uTime*3.0);
  vec3 c = mix(mag, cyn, step(0.5, fract(b.y/scale*0.18 - uTime*0.6 + k*0.5)));
  float led = 0.65 + 0.35*smoothstep(0.2, 0.0, abs(seg - 0.5) - 0.3);
  return vec3(1.0)*exp(-pow(d/0.28, 2.0))*1.3 + c*halo(d, 0.75)*0.6*led;
}
vec3 infinity_mirror_glow0(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float on = smoothstep(-0.5, 0.3, b.y);
  vec3 light = im_blade(b, px, 0.0, 1.0)*on;
  // the endless reflections: copies on both sides, sliding outward, each smaller (perspective)
  float slide = fract(uTime*(0.35));
  for (int s = 0; s < 2; s++){
    float sd = s == 0 ? 1.0 : -1.0;
    for (int k = 0; k < 7; k++){
      float n = float(k) + slide;
      float scale = pow(0.8, n);
      float x0 = sd*(1.6*(1.0 - pow(0.8, n))/0.2)*0.95;
      float dim = pow(0.72, n)*smoothstep(0.0, 0.6, n);
      vec2 q = vec2(b.x - x0, b.y);
      light += im_blade(q, px, n, scale)*dim*on*smoothstep(-0.3, 0.6, b.y);
    }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.1))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }

// ==== show (added) ====
// Infinity Mirror show: reflection frames peel off the blade and recede outward like an endless
// mirror tunnel, LED lights chasing round each frame in magenta and cyan, and mirror shards with
// sliding reflections drifting away; a swing sends frames flying out faster, a clash flashes one huge
// frame and a spray of shards.
float imBox(vec2 q, vec2 h, float rr){ vec2 d = abs(q) - h + rr; return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0) - rr; }
vec3 infinity_mirror_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 mag = vec3(1.0, 0.3, 0.85), cyn = vec3(0.3, 0.9, 1.0);
  vec3 light = vec3(0.0);
  // receding frames: rounded rectangles round blade sections, growing outward and fading
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float f = fract(uTime*(0.25) + fi/6.0);
    float yc = (0.2 + 0.6*fract(fi*0.618))*L;
    vec2 h = vec2(0.9 + 6.5*f, 1.6 + 3.0*f);
    vec2 q = b - vec2(0.0, yc);
    float d = abs(imBox(q, h, 0.6));
    float per = 2.0*(h.x + h.y);
    float along = (atan(q.y, q.x) + 3.1416)/6.2832;
    float leds = smoothstep(0.75, 1.0, sin(along*per*2.5 - uTime*8.0));
    vec3 c = mix(mag, cyn, step(0.5, fract(along*6.0 + fi*0.5)));
    float fade = smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.6, 1.0, f));
    light += c*(exp(-pow(d/0.04, 2.0))*0.7 + leds*halo(d, 0.08)*1.6)*fade;
  }
  // mirror shards drifting away, a reflection sliding across each
  for (int i = 0; i < 9; i++){
    float fi = float(i);
    float ph = uTime*(0.25 + 0.15*h11(fi*3.3)) + h11(fi*8.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*2.7 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 p = vec2(s*(0.8 + 6.0*f), (0.1 + 0.85*h11(cy*1.9 + fi))*L + 1.5*f);
    vec2 q = b - p;
    float an = uTime*1.3 + fi;
    q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
    float sh = imBox(q + vec2(q.y*0.4, 0.0), vec2(0.28, 0.45), 0.03);
    float refl = halo(abs(q.x + q.y*0.6 - sin(uTime*2.0 + fi)*0.4), 0.06);
    float fade = (1.0 - f)*smoothstep(0.0, 0.08, f);
    light += (mix(mag, cyn, h11(fi))*halo(abs(sh), 0.025 + px)*1.3 + vec3(1.0)*refl*step(sh, 0.0)*0.8 + vec3(0.25, 0.2, 0.35)*step(sh, 0.0))*fade;
  }
  // clash: one huge frame flashes out, shards spray
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 q = b - vec2(0.0, L*0.6);
    float d = abs(imBox(q, vec2(1.0 + 7.0*a, 2.0 + 5.0*a), 0.8));
    light += mix(mag, cyn, 0.5)*halo(d, 0.1)*cl*1.6 + vec3(1.0)*halo(length(q), 1.5)*pow(cl, 3.0);
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return infinity_mirror_glow0(g, b, r, px) + infinity_mirror_show(b, min(px, 0.06));
}
