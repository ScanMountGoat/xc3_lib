const int undef = 0;
layout(binding = 0, std140) uniform _support_buffer {
    uint alpha_test;
    uint is_bgra[8];
    precise vec4 viewport_inverse;
    precise vec4 viewport_size;
    int frag_scale_count;
    precise float render_scale[73];
    ivec4 tfe_offset;
    int tfe_vertex_count;
}support_buffer;
layout(binding = 10, std140) uniform _U_RimBloomCalc {
    vec4 gRimBloomCalcWork[2];
}U_RimBloomCalc;
layout(binding = 9, std140) uniform _U_VolTexCalc {
    vec4 gVolTexCalcWork[10];
}U_VolTexCalc;
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gWrkFl4[5];
}U_Mate;
layout(binding = 11, std140) uniform _U_Toon2 {
    vec4 gToonParam[4];
}U_Toon2;
layout(binding = 6, std140) uniform _U_LGT {
    vec4 gLgtPreDir[2];
    vec4 gLgtPreCol[2];
    vec4 gLgtPreAmb;
    vec4 gLgtNoUse;
    vec4 gMatSH[12];
    vec4 gMipCount;
}U_LGT;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(binding = 4, std140) uniform _U_Static {
    vec4 gmView[3];
    vec4 gmProj[4];
    vec4 gmViewProj[4];
    vec4 gmInvView[3];
    vec4 gBilMat[3];
    vec4 gBilYJiku[3];
    vec4 gEtcParm;
    vec4 gViewYVec;
    vec4 gCDep;
    vec4 gDitVal;
    vec4 gPreMat[4];
    vec4 gScreenSize;
    vec4 gJitter;
    vec4 gDitTMAAVal;
    vec4 gmProjNonJitter[4];
    vec4 gmDiffPreMat[4];
}U_Static;
layout(binding = 0) uniform sampler3D volTex0;
layout(binding = 1) uniform sampler2D s0;
layout(binding = 2) uniform sampler2D s1;
layout(binding = 3) uniform sampler2D texAO;
layout(binding = 4) uniform sampler2D texLgt;
layout(binding = 5) uniform sampler2D texShadow;
layout(binding = 6) uniform sampler2D gTToonDarkGrad;
layout(binding = 7) uniform sampler2D gTToonGrad;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
layout(location = 4) in vec4 in_attr4;
layout(location = 5) in vec4 in_attr5;
layout(location = 6) in vec4 in_attr6;
layout(location = 7) in vec4 in_attr7;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
void main() {
    bool temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise float temp_4;
    precise float temp_5;
    precise float temp_6;
    precise float temp_7;
    precise float temp_8;
    precise float temp_9;
    precise float temp_10;
    precise float temp_11;
    precise float temp_12;
    precise float temp_13;
    precise float temp_14;
    precise float temp_15;
    precise float temp_16;
    precise float temp_17;
    precise float temp_18;
    precise float temp_19;
    precise float temp_20;
    precise float temp_21;
    precise float temp_22;
    precise float temp_23;
    precise float temp_24;
    precise float temp_25;
    precise float temp_26;
    precise float temp_27;
    precise float temp_28;
    precise float temp_29;
    precise float temp_30;
    precise float temp_31;
    precise float temp_32;
    precise float temp_33;
    precise float temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise float temp_38;
    precise float temp_39;
    precise float temp_40;
    precise float temp_41;
    precise float temp_42;
    precise float temp_43;
    precise float temp_44;
    precise float temp_45;
    precise float temp_46;
    precise float temp_47;
    precise float temp_48;
    precise float temp_49;
    precise float temp_50;
    precise float temp_51;
    precise float temp_52;
    precise float temp_53;
    precise float temp_54;
    precise float temp_55;
    precise float temp_56;
    precise float temp_57;
    precise float temp_58;
    precise float temp_59;
    precise float temp_60;
    precise float temp_61;
    precise float temp_62;
    precise float temp_63;
    bool temp_64;
    precise float temp_65;
    precise vec3 temp_66;
    precise float temp_67;
    precise float temp_68;
    precise float temp_69;
    precise float temp_70;
    precise float temp_71;
    precise vec3 temp_72;
    precise float temp_73;
    precise float temp_74;
    precise float temp_75;
    precise float temp_76;
    precise float temp_77;
    precise float temp_78;
    precise float temp_79;
    precise float temp_80;
    precise float temp_81;
    precise float temp_82;
    precise float temp_83;
    precise float temp_84;
    precise float temp_85;
    precise float temp_86;
    precise float temp_87;
    precise float temp_88;
    precise float temp_89;
    precise float temp_90;
    precise float temp_91;
    precise float temp_92;
    precise float temp_93;
    precise float temp_94;
    precise float temp_95;
    precise float temp_96;
    precise float temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    precise float temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    precise float temp_106;
    precise float temp_107;
    precise float temp_108;
    precise float temp_109;
    precise float temp_110;
    bool temp_111;
    precise float temp_112;
    precise float temp_113;
    precise float temp_114;
    bool temp_115;
    bool temp_116;
    precise float temp_117;
    precise float temp_118;
    precise float temp_119;
    precise float temp_120;
    precise float temp_121;
    precise float temp_122;
    precise float temp_123;
    precise float temp_124;
    precise float temp_125;
    precise float temp_126;
    precise float temp_127;
    precise float temp_128;
    precise float temp_129;
    precise float temp_130;
    precise float temp_131;
    precise float temp_132;
    precise float temp_133;
    precise float temp_134;
    precise float temp_135;
    precise float temp_136;
    precise float temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    precise float temp_141;
    precise float temp_142;
    precise float temp_143;
    precise float temp_144;
    precise float temp_145;
    precise float temp_146;
    precise float temp_147;
    precise float temp_148;
    precise float temp_149;
    precise float temp_150;
    precise float temp_151;
    precise float temp_152;
    precise float temp_153;
    precise float temp_154;
    precise float temp_155;
    precise float temp_156;
    precise float temp_157;
    precise float temp_158;
    precise float temp_159;
    precise float temp_160;
    precise float temp_161;
    precise float temp_162;
    precise float temp_163;
    precise float temp_164;
    bool temp_165;
    precise float temp_166;
    precise float temp_167;
    precise float temp_168;
    precise float temp_169;
    precise float temp_170;
    precise float temp_171;
    precise float temp_172;
    precise float temp_173;
    precise float temp_174;
    precise float temp_175;
    precise float temp_176;
    precise float temp_177;
    precise float temp_178;
    precise float temp_179;
    precise float temp_180;
    precise float temp_181;
    precise float temp_182;
    precise float temp_183;
    bool temp_184;
    precise float temp_185;
    precise float temp_186;
    precise float temp_187;
    precise float temp_188;
    bool temp_189;
    precise float temp_190;
    precise float temp_191;
    precise float temp_192;
    precise float temp_193;
    precise float temp_194;
    precise float temp_195;
    precise float temp_196;
    bool temp_197;
    bool temp_198;
    bool temp_199;
    precise float temp_200;
    precise float temp_201;
    precise float temp_202;
    precise float temp_203;
    precise float temp_204;
    precise float temp_205;
    precise float temp_206;
    precise float temp_207;
    precise float temp_208;
    precise float temp_209;
    precise float temp_210;
    bool temp_211;
    precise float temp_212;
    precise float temp_213;
    precise float temp_214;
    precise float temp_215;
    precise float temp_216;
    precise float temp_217;
    precise float temp_218;
    precise float temp_219;
    precise float temp_220;
    precise float temp_221;
    precise float temp_222;
    precise float temp_223;
    precise float temp_224;
    precise float temp_225;
    precise float temp_226;
    precise float temp_227;
    precise float temp_228;
    precise float temp_229;
    bool temp_230;
    precise float temp_231;
    precise float temp_232;
    precise float temp_233;
    precise float temp_234;
    bool temp_235;
    precise float temp_236;
    precise float temp_237;
    precise float temp_238;
    precise float temp_239;
    precise float temp_240;
    precise float temp_241;
    precise float temp_242;
    bool temp_243;
    bool temp_244;
    bool temp_245;
    precise float temp_246;
    precise float temp_247;
    precise float temp_248;
    precise float temp_249;
    precise float temp_250;
    precise float temp_251;
    precise float temp_252;
    precise float temp_253;
    precise float temp_254;
    precise float temp_255;
    precise float temp_256;
    bool temp_257;
    precise float temp_258;
    precise float temp_259;
    precise float temp_260;
    precise float temp_261;
    precise float temp_262;
    precise float temp_263;
    precise float temp_264;
    precise float temp_265;
    precise float temp_266;
    precise float temp_267;
    precise float temp_268;
    precise float temp_269;
    precise float temp_270;
    precise float temp_271;
    precise float temp_272;
    precise float temp_273;
    precise float temp_274;
    precise float temp_275;
    bool temp_276;
    precise float temp_277;
    precise float temp_278;
    precise float temp_279;
    precise float temp_280;
    bool temp_281;
    precise float temp_282;
    precise float temp_283;
    precise float temp_284;
    precise float temp_285;
    precise float temp_286;
    precise float temp_287;
    precise float temp_288;
    bool temp_289;
    bool temp_290;
    bool temp_291;
    precise float temp_292;
    precise float temp_293;
    precise float temp_294;
    precise float temp_295;
    precise float temp_296;
    precise float temp_297;
    precise float temp_298;
    precise float temp_299;
    precise float temp_300;
    precise float temp_301;
    precise float temp_302;
    bool temp_303;
    precise float temp_304;
    precise float temp_305;
    precise float temp_306;
    precise float temp_307;
    precise float temp_308;
    precise float temp_309;
    precise float temp_310;
    precise float temp_311;
    precise float temp_312;
    precise float temp_313;
    precise float temp_314;
    precise float temp_315;
    precise float temp_316;
    precise float temp_317;
    precise float temp_318;
    precise float temp_319;
    precise float temp_320;
    precise float temp_321;
    bool temp_322;
    precise float temp_323;
    precise float temp_324;
    precise float temp_325;
    precise float temp_326;
    bool temp_327;
    precise float temp_328;
    precise float temp_329;
    precise float temp_330;
    precise float temp_331;
    precise float temp_332;
    precise float temp_333;
    precise float temp_334;
    bool temp_335;
    bool temp_336;
    precise float temp_337;
    precise float temp_338;
    precise float temp_339;
    precise float temp_340;
    precise float temp_341;
    precise float temp_342;
    bool temp_343;
    precise float temp_344;
    precise float temp_345;
    precise float temp_346;
    precise float temp_347;
    precise float temp_348;
    precise float temp_349;
    precise float temp_350;
    precise float temp_351;
    precise float temp_352;
    precise float temp_353;
    precise float temp_354;
    precise float temp_355;
    precise float temp_356;
    precise float temp_357;
    precise float temp_358;
    precise float temp_359;
    precise float temp_360;
    precise float temp_361;
    precise float temp_362;
    precise float temp_363;
    precise float temp_364;
    precise float temp_365;
    precise float temp_366;
    precise float temp_367;
    precise float temp_368;
    precise float temp_369;
    int temp_370;
    precise float temp_371;
    precise float temp_372;
    precise float temp_373;
    precise float temp_374;
    precise vec3 temp_375;
    precise float temp_376;
    precise float temp_377;
    precise float temp_378;
    precise vec3 temp_379;
    precise float temp_380;
    precise float temp_381;
    precise float temp_382;
    precise float temp_383;
    precise float temp_384;
    precise float temp_385;
    precise float temp_386;
    precise float temp_387;
    precise float temp_388;
    precise float temp_389;
    precise float temp_390;
    precise float temp_391;
    precise float temp_392;
    precise float temp_393;
    precise float temp_394;
    precise float temp_395;
    precise float temp_396;
    precise float temp_397;
    precise float temp_398;
    precise float temp_399;
    precise float temp_400;
    precise float temp_401;
    precise float temp_402;
    precise float temp_403;
    precise float temp_404;
    precise float temp_405;
    precise float temp_406;
    precise float temp_407;
    precise float temp_408;
    precise float temp_409;
    precise float temp_410;
    precise float temp_411;
    precise float temp_412;
    precise float temp_413;
    precise float temp_414;
    precise float temp_415;
    precise float temp_416;
    precise float temp_417;
    precise float temp_418;
    precise float temp_419;
    precise float temp_420;
    precise float temp_421;
    precise float temp_422;
    precise float temp_423;
    precise float temp_424;
    precise float temp_425;
    precise float temp_426;
    precise float temp_427;
    precise float temp_428;
    precise float temp_429;
    precise float temp_430;
    precise float temp_431;
    precise float temp_432;
    precise float temp_433;
    precise float temp_434;
    precise float temp_435;
    precise float temp_436;
    precise float temp_437;
    precise float temp_438;
    precise float temp_439;
    precise float temp_440;
    precise float temp_441;
    precise float temp_442;
    precise float temp_443;
    precise float temp_444;
    precise float temp_445;
    precise float temp_446;
    precise float temp_447;
    precise float temp_448;
    precise float temp_449;
    precise float temp_450;
    precise float temp_451;
    precise float temp_452;
    precise float temp_453;
    precise float temp_454;
    precise float temp_455;
    precise float temp_456;
    precise float temp_457;
    precise float temp_458;
    precise float temp_459;
    precise float temp_460;
    precise float temp_461;
    precise float temp_462;
    precise float temp_463;
    precise float temp_464;
    precise float temp_465;
    precise float temp_466;
    precise float temp_467;
    precise float temp_468;
    precise float temp_469;
    precise float temp_470;
    precise float temp_471;
    temp_0 = 0. < U_RimBloomCalc.gRimBloomCalcWork[1].z;
    temp_1 = in_attr7.z;
    temp_2 = in_attr7.x;
    temp_3 = in_attr7.y;
    temp_4 = temp_1 * U_VolTexCalc.gVolTexCalcWork[0].z;
    temp_5 = temp_2 * U_VolTexCalc.gVolTexCalcWork[0].x;
    temp_6 = temp_3 * U_VolTexCalc.gVolTexCalcWork[0].y;
    temp_7 = texture(volTex0, vec3(temp_5, temp_6, temp_4)).x;
    temp_8 = 0. - U_VolTexCalc.gVolTexCalcWork[1].x;
    temp_9 = temp_2 + temp_8;
    temp_10 = 1. / U_VolTexCalc.gVolTexCalcWork[4].z;
    temp_11 = 0. - U_VolTexCalc.gVolTexCalcWork[1].y;
    temp_12 = temp_3 + temp_11;
    temp_13 = in_attr0.y;
    temp_14 = temp_9 * temp_9;
    temp_15 = in_attr0.z;
    temp_16 = 0. - U_VolTexCalc.gVolTexCalcWork[1].z;
    temp_17 = temp_1 + temp_16;
    temp_18 = in_attr1.y;
    temp_19 = fma(temp_12, temp_12, temp_14);
    temp_20 = in_attr1.z;
    temp_21 = fma(temp_17, temp_17, temp_19);
    temp_22 = in_attr6.w;
    temp_23 = in_attr3.z;
    temp_24 = sqrt(temp_21);
    temp_25 = U_VolTexCalc.gVolTexCalcWork[4].w + U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_26 = in_attr3.w;
    temp_27 = min(temp_24, U_VolTexCalc.gVolTexCalcWork[4].z);
    temp_28 = in_attr0.x;
    temp_29 = fma(temp_25, U_VolTexCalc.gVolTexCalcWork[0].w, U_VolTexCalc.gVolTexCalcWork[0].w);
    temp_30 = temp_27 * temp_10;
    temp_31 = 0. - U_VolTexCalc.gVolTexCalcWork[4].w;
    temp_32 = fma(temp_30, temp_31, temp_29);
    temp_33 = 1. / temp_22;
    temp_34 = 0. - U_VolTexCalc.gVolTexCalcWork[1].w;
    temp_35 = temp_32 + temp_34;
    temp_36 = in_attr1.x;
    temp_37 = temp_28 * temp_28;
    temp_38 = fma(temp_13, temp_13, temp_37);
    temp_39 = fma(temp_15, temp_15, temp_38);
    temp_40 = temp_36 * temp_36;
    temp_41 = inversesqrt(temp_39);
    temp_42 = fma(temp_18, temp_18, temp_40);
    temp_43 = fma(temp_20, temp_20, temp_42);
    temp_44 = in_attr3.y;
    temp_45 = temp_28 * temp_41;
    temp_46 = inversesqrt(temp_43);
    temp_47 = temp_13 * temp_41;
    temp_48 = temp_15 * temp_41;
    temp_49 = in_attr3.x;
    temp_50 = temp_36 * temp_46;
    temp_51 = intBitsToFloat(undef);
    temp_52 = temp_46;
    temp_53 = temp_47;
    temp_54 = temp_48;
    if (temp_0) {
        temp_55 = 0. - temp_50;
        temp_56 = temp_45 * temp_55;
        temp_51 = temp_56;
    }
    temp_57 = temp_51;
    temp_58 = in_attr6.x;
    temp_59 = in_attr6.y;
    temp_60 = temp_58 * temp_33;
    temp_61 = temp_59 * temp_33;
    temp_62 = fma(temp_60, 0.5, 0.5);
    temp_63 = fma(temp_61, -0.5, 0.5);
    temp_64 = temp_7 < temp_35;
    temp_65 = temp_57;
    if (temp_64) {
        discard;
    }
    temp_66 = texture(s0, vec2(temp_49, temp_44)).xyz;
    temp_67 = temp_66.x;
    temp_68 = temp_66.y;
    temp_69 = temp_66.z;
    temp_70 = texture(s1, vec2(temp_23, temp_26)).x;
    temp_71 = texture(texAO, vec2(temp_62, temp_63)).z;
    temp_72 = texture(texLgt, vec2(temp_62, temp_63)).xyz;
    temp_73 = temp_72.x;
    temp_74 = temp_72.y;
    temp_75 = temp_72.z;
    temp_76 = temp_18 * temp_46;
    temp_77 = temp_20 * temp_46;
    temp_78 = temp_77;
    temp_79 = temp_67;
    temp_80 = temp_68;
    temp_81 = temp_69;
    if (temp_0) {
        temp_82 = 0. - temp_76;
        temp_83 = fma(temp_47, temp_82, temp_57);
        temp_65 = temp_83;
    }
    temp_84 = temp_65;
    temp_85 = temp_84;
    if (temp_0) {
        temp_86 = 0. - temp_77;
        temp_87 = fma(temp_48, temp_86, temp_84);
        temp_85 = temp_87;
    }
    temp_88 = temp_85;
    temp_89 = temp_88;
    if (temp_0) {
        temp_90 = abs(temp_88);
        temp_91 = 0. - temp_90;
        temp_92 = temp_91 + 1.;
        temp_52 = temp_92;
    }
    temp_93 = temp_52;
    temp_94 = temp_93;
    if (temp_0) {
        temp_89 = U_RimBloomCalc.gRimBloomCalcWork[1].x;
    }
    temp_95 = temp_89;
    temp_96 = temp_95;
    if (temp_0) {
        temp_97 = log2(temp_93);
        temp_94 = temp_97;
    }
    temp_98 = temp_94;
    temp_99 = temp_98;
    if (temp_0) {
        temp_100 = temp_95 * 10.;
        temp_96 = temp_100;
    }
    temp_101 = temp_96;
    temp_102 = temp_101;
    if (temp_0) {
        temp_103 = temp_101 * temp_98;
        temp_102 = temp_103;
    }
    temp_104 = temp_102;
    temp_105 = intBitsToFloat(undef);
    temp_106 = temp_104;
    if (temp_0) {
        temp_105 = temp_104;
    }
    temp_107 = temp_105;
    if (temp_0) {
        temp_108 = exp2(temp_107);
        temp_106 = temp_108;
    }
    temp_109 = temp_106;
    temp_110 = temp_35 + U_VolTexCalc.gVolTexCalcWork[4].y;
    temp_111 = temp_7 < temp_110;
    temp_112 = temp_109;
    if (temp_0) {
        temp_113 = temp_109 * U_RimBloomCalc.gRimBloomCalcWork[1].z;
        temp_112 = temp_113;
    }
    temp_114 = temp_112;
    temp_115 = !temp_111;
    temp_116 = temp_115;
    if (temp_0) {
        temp_117 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_118 = fma(temp_67, temp_117, temp_67);
        temp_53 = temp_118;
    }
    temp_119 = temp_53;
    temp_120 = temp_119;
    if (temp_0) {
        temp_121 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_122 = fma(temp_68, temp_121, temp_68);
        temp_78 = temp_122;
    }
    temp_123 = temp_78;
    if (temp_0) {
        temp_124 = 0. - temp_119;
        temp_125 = temp_124 + U_RimBloomCalc.gRimBloomCalcWork[0].x;
        temp_99 = temp_125;
    }
    temp_126 = temp_99;
    if (temp_0) {
        temp_127 = 0. - temp_123;
        temp_128 = temp_127 + U_RimBloomCalc.gRimBloomCalcWork[0].y;
        temp_54 = temp_128;
    }
    temp_129 = temp_54;
    if (temp_0) {
        temp_130 = fma(temp_126, temp_114, temp_119);
        temp_79 = temp_130;
    }
    temp_131 = temp_79;
    if (temp_0) {
        temp_132 = 0. - U_RimBloomCalc.gRimBloomCalcWork[1].y;
        temp_133 = fma(temp_69, temp_132, temp_69);
        temp_120 = temp_133;
    }
    temp_134 = temp_120;
    temp_135 = temp_69;
    if (temp_0) {
        temp_136 = fma(temp_129, temp_114, temp_123);
        temp_80 = temp_136;
    }
    temp_137 = temp_80;
    if (temp_0) {
        temp_138 = 0. - temp_134;
        temp_139 = temp_138 + U_RimBloomCalc.gRimBloomCalcWork[0].z;
        temp_81 = temp_139;
    }
    temp_140 = temp_81;
    temp_141 = 0.;
    if (temp_0) {
        temp_141 = temp_114;
    }
    temp_142 = temp_141;
    if (temp_0) {
        temp_143 = fma(temp_140, temp_114, temp_134);
        temp_135 = temp_143;
    }
    temp_144 = temp_135;
    temp_145 = temp_35;
    temp_146 = temp_142;
    temp_147 = temp_131;
    temp_148 = temp_137;
    if (temp_115) {
        temp_145 = temp_110;
    }
    temp_149 = temp_145;
    temp_150 = temp_144;
    temp_151 = temp_149;
    if (temp_111) {
        temp_146 = U_VolTexCalc.gVolTexCalcWork[9].w;
    }
    temp_152 = temp_146;
    temp_153 = temp_152;
    temp_154 = temp_152;
    if (temp_111) {
        temp_147 = U_VolTexCalc.gVolTexCalcWork[9].x;
    }
    temp_155 = temp_147;
    temp_156 = temp_155;
    temp_157 = temp_155;
    if (temp_111) {
        temp_148 = U_VolTexCalc.gVolTexCalcWork[9].y;
    }
    temp_158 = temp_148;
    temp_159 = temp_158;
    temp_160 = temp_158;
    if (temp_111) {
        temp_150 = U_VolTexCalc.gVolTexCalcWork[9].z;
    }
    temp_161 = temp_150;
    temp_162 = temp_161;
    temp_163 = temp_161;
    if (temp_115) {
        temp_164 = temp_149 + U_VolTexCalc.gVolTexCalcWork[4].x;
        temp_165 = temp_7 < temp_164;
        if (temp_165) {
            temp_166 = 0. - temp_149;
            temp_167 = temp_166 + temp_164;
            temp_168 = 1. / temp_167;
            temp_169 = 0. - temp_149;
            temp_170 = temp_169 + temp_7;
            temp_171 = 0. - U_VolTexCalc.gVolTexCalcWork[9].x;
            temp_172 = temp_171 + U_VolTexCalc.gVolTexCalcWork[8].x;
            temp_173 = 0. - U_VolTexCalc.gVolTexCalcWork[9].y;
            temp_174 = temp_173 + U_VolTexCalc.gVolTexCalcWork[8].y;
            temp_175 = 0. - U_VolTexCalc.gVolTexCalcWork[9].z;
            temp_176 = temp_175 + U_VolTexCalc.gVolTexCalcWork[8].z;
            temp_177 = 0. - U_VolTexCalc.gVolTexCalcWork[9].w;
            temp_178 = temp_177 + U_VolTexCalc.gVolTexCalcWork[8].w;
            temp_179 = temp_170 * temp_168;
            temp_180 = fma(temp_179, temp_172, U_VolTexCalc.gVolTexCalcWork[9].x);
            temp_181 = fma(temp_179, temp_174, U_VolTexCalc.gVolTexCalcWork[9].y);
            temp_182 = fma(temp_179, temp_176, U_VolTexCalc.gVolTexCalcWork[9].z);
            temp_183 = fma(temp_179, temp_178, U_VolTexCalc.gVolTexCalcWork[9].w);
            temp_116 = false;
            temp_156 = temp_180;
            temp_159 = temp_181;
            temp_162 = temp_182;
            temp_153 = temp_183;
        }
        temp_184 = temp_116;
        temp_185 = temp_156;
        temp_186 = temp_159;
        temp_187 = temp_162;
        temp_188 = temp_153;
        temp_189 = temp_184;
        temp_190 = temp_185;
        temp_191 = temp_186;
        temp_192 = temp_187;
        temp_193 = temp_188;
        temp_157 = temp_185;
        temp_160 = temp_186;
        temp_163 = temp_187;
        temp_154 = temp_188;
        if (temp_184) {
            temp_151 = temp_164;
        }
        temp_194 = temp_151;
        temp_195 = temp_194;
        if (temp_184) {
            temp_196 = temp_194 + U_VolTexCalc.gVolTexCalcWork[3].w;
            temp_197 = temp_7 < temp_196;
            if (temp_197) {
                temp_189 = false;
            }
            temp_198 = temp_189;
            temp_199 = temp_198;
            if (temp_197) {
                temp_190 = U_VolTexCalc.gVolTexCalcWork[8].x;
            }
            temp_200 = temp_190;
            temp_201 = temp_200;
            temp_157 = temp_200;
            if (temp_197) {
                temp_191 = U_VolTexCalc.gVolTexCalcWork[8].y;
            }
            temp_202 = temp_191;
            temp_203 = temp_202;
            temp_160 = temp_202;
            if (temp_197) {
                temp_192 = U_VolTexCalc.gVolTexCalcWork[8].z;
            }
            temp_204 = temp_192;
            temp_205 = temp_204;
            temp_163 = temp_204;
            if (temp_197) {
                temp_193 = U_VolTexCalc.gVolTexCalcWork[8].w;
            }
            temp_206 = temp_193;
            temp_207 = temp_206;
            temp_154 = temp_206;
            if (temp_198) {
                temp_195 = temp_196;
            }
            temp_208 = temp_195;
            temp_209 = temp_208;
            if (temp_198) {
                temp_210 = temp_208 + U_VolTexCalc.gVolTexCalcWork[3].z;
                temp_211 = temp_7 < temp_210;
                if (temp_211) {
                    temp_212 = 0. - temp_208;
                    temp_213 = temp_212 + temp_210;
                    temp_214 = 1. / temp_213;
                    temp_215 = 0. - temp_208;
                    temp_216 = temp_215 + temp_7;
                    temp_217 = temp_216 * temp_214;
                    temp_218 = 0. - U_VolTexCalc.gVolTexCalcWork[8].x;
                    temp_219 = temp_218 + U_VolTexCalc.gVolTexCalcWork[7].x;
                    temp_220 = 0. - U_VolTexCalc.gVolTexCalcWork[8].y;
                    temp_221 = temp_220 + U_VolTexCalc.gVolTexCalcWork[7].y;
                    temp_222 = 0. - U_VolTexCalc.gVolTexCalcWork[8].z;
                    temp_223 = temp_222 + U_VolTexCalc.gVolTexCalcWork[7].z;
                    temp_224 = 0. - U_VolTexCalc.gVolTexCalcWork[8].w;
                    temp_225 = temp_224 + U_VolTexCalc.gVolTexCalcWork[7].w;
                    temp_226 = fma(temp_217, temp_219, U_VolTexCalc.gVolTexCalcWork[8].x);
                    temp_227 = fma(temp_217, temp_221, U_VolTexCalc.gVolTexCalcWork[8].y);
                    temp_228 = fma(temp_217, temp_223, U_VolTexCalc.gVolTexCalcWork[8].z);
                    temp_229 = fma(temp_217, temp_225, U_VolTexCalc.gVolTexCalcWork[8].w);
                    temp_199 = false;
                    temp_201 = temp_226;
                    temp_203 = temp_227;
                    temp_205 = temp_228;
                    temp_207 = temp_229;
                }
                temp_230 = temp_199;
                temp_231 = temp_201;
                temp_232 = temp_203;
                temp_233 = temp_205;
                temp_234 = temp_207;
                temp_235 = temp_230;
                temp_236 = temp_231;
                temp_237 = temp_232;
                temp_238 = temp_233;
                temp_239 = temp_234;
                temp_157 = temp_231;
                temp_160 = temp_232;
                temp_163 = temp_233;
                temp_154 = temp_234;
                if (temp_230) {
                    temp_209 = temp_210;
                }
                temp_240 = temp_209;
                temp_241 = temp_240;
                if (temp_230) {
                    temp_242 = temp_240 + U_VolTexCalc.gVolTexCalcWork[3].y;
                    temp_243 = temp_7 < temp_242;
                    if (temp_243) {
                        temp_235 = false;
                    }
                    temp_244 = temp_235;
                    temp_245 = temp_244;
                    if (temp_243) {
                        temp_236 = U_VolTexCalc.gVolTexCalcWork[7].x;
                    }
                    temp_246 = temp_236;
                    temp_247 = temp_246;
                    temp_157 = temp_246;
                    if (temp_243) {
                        temp_237 = U_VolTexCalc.gVolTexCalcWork[7].y;
                    }
                    temp_248 = temp_237;
                    temp_249 = temp_248;
                    temp_160 = temp_248;
                    if (temp_243) {
                        temp_238 = U_VolTexCalc.gVolTexCalcWork[7].z;
                    }
                    temp_250 = temp_238;
                    temp_251 = temp_250;
                    temp_163 = temp_250;
                    if (temp_243) {
                        temp_239 = U_VolTexCalc.gVolTexCalcWork[7].w;
                    }
                    temp_252 = temp_239;
                    temp_253 = temp_252;
                    temp_154 = temp_252;
                    if (temp_244) {
                        temp_241 = temp_242;
                    }
                    temp_254 = temp_241;
                    temp_255 = temp_254;
                    if (temp_244) {
                        temp_256 = temp_254 + U_VolTexCalc.gVolTexCalcWork[3].x;
                        temp_257 = temp_7 < temp_256;
                        if (temp_257) {
                            temp_258 = 0. - temp_254;
                            temp_259 = temp_258 + temp_256;
                            temp_260 = 1. / temp_259;
                            temp_261 = 0. - temp_254;
                            temp_262 = temp_261 + temp_7;
                            temp_263 = temp_262 * temp_260;
                            temp_264 = 0. - U_VolTexCalc.gVolTexCalcWork[7].x;
                            temp_265 = temp_264 + U_VolTexCalc.gVolTexCalcWork[6].x;
                            temp_266 = 0. - U_VolTexCalc.gVolTexCalcWork[7].y;
                            temp_267 = temp_266 + U_VolTexCalc.gVolTexCalcWork[6].y;
                            temp_268 = 0. - U_VolTexCalc.gVolTexCalcWork[7].z;
                            temp_269 = temp_268 + U_VolTexCalc.gVolTexCalcWork[6].z;
                            temp_270 = 0. - U_VolTexCalc.gVolTexCalcWork[7].w;
                            temp_271 = temp_270 + U_VolTexCalc.gVolTexCalcWork[6].w;
                            temp_272 = fma(temp_263, temp_265, U_VolTexCalc.gVolTexCalcWork[7].x);
                            temp_273 = fma(temp_263, temp_267, U_VolTexCalc.gVolTexCalcWork[7].y);
                            temp_274 = fma(temp_263, temp_269, U_VolTexCalc.gVolTexCalcWork[7].z);
                            temp_275 = fma(temp_263, temp_271, U_VolTexCalc.gVolTexCalcWork[7].w);
                            temp_245 = false;
                            temp_247 = temp_272;
                            temp_249 = temp_273;
                            temp_251 = temp_274;
                            temp_253 = temp_275;
                        }
                        temp_276 = temp_245;
                        temp_277 = temp_247;
                        temp_278 = temp_249;
                        temp_279 = temp_251;
                        temp_280 = temp_253;
                        temp_281 = temp_276;
                        temp_282 = temp_277;
                        temp_283 = temp_278;
                        temp_284 = temp_279;
                        temp_285 = temp_280;
                        temp_157 = temp_277;
                        temp_160 = temp_278;
                        temp_163 = temp_279;
                        temp_154 = temp_280;
                        if (temp_276) {
                            temp_255 = temp_256;
                        }
                        temp_286 = temp_255;
                        temp_287 = temp_286;
                        if (temp_276) {
                            temp_288 = temp_286 + U_VolTexCalc.gVolTexCalcWork[2].w;
                            temp_289 = temp_7 < temp_288;
                            if (temp_289) {
                                temp_281 = false;
                            }
                            temp_290 = temp_281;
                            temp_291 = temp_290;
                            if (temp_289) {
                                temp_282 = U_VolTexCalc.gVolTexCalcWork[6].x;
                            }
                            temp_292 = temp_282;
                            temp_293 = temp_292;
                            temp_157 = temp_292;
                            if (temp_289) {
                                temp_283 = U_VolTexCalc.gVolTexCalcWork[6].y;
                            }
                            temp_294 = temp_283;
                            temp_295 = temp_294;
                            temp_160 = temp_294;
                            if (temp_289) {
                                temp_284 = U_VolTexCalc.gVolTexCalcWork[6].z;
                            }
                            temp_296 = temp_284;
                            temp_297 = temp_296;
                            temp_163 = temp_296;
                            if (temp_289) {
                                temp_285 = U_VolTexCalc.gVolTexCalcWork[6].w;
                            }
                            temp_298 = temp_285;
                            temp_299 = temp_298;
                            temp_154 = temp_298;
                            if (temp_290) {
                                temp_287 = temp_288;
                            }
                            temp_300 = temp_287;
                            temp_301 = temp_300;
                            if (temp_290) {
                                temp_302 = temp_300 + U_VolTexCalc.gVolTexCalcWork[2].z;
                                temp_303 = temp_7 < temp_302;
                                if (temp_303) {
                                    temp_304 = 0. - temp_300;
                                    temp_305 = temp_304 + temp_302;
                                    temp_306 = 1. / temp_305;
                                    temp_307 = 0. - temp_300;
                                    temp_308 = temp_307 + temp_7;
                                    temp_309 = temp_308 * temp_306;
                                    temp_310 = 0. - U_VolTexCalc.gVolTexCalcWork[6].x;
                                    temp_311 = temp_310 + U_VolTexCalc.gVolTexCalcWork[5].x;
                                    temp_312 = 0. - U_VolTexCalc.gVolTexCalcWork[6].y;
                                    temp_313 = temp_312 + U_VolTexCalc.gVolTexCalcWork[5].y;
                                    temp_314 = 0. - U_VolTexCalc.gVolTexCalcWork[6].z;
                                    temp_315 = temp_314 + U_VolTexCalc.gVolTexCalcWork[5].z;
                                    temp_316 = 0. - U_VolTexCalc.gVolTexCalcWork[6].w;
                                    temp_317 = temp_316 + U_VolTexCalc.gVolTexCalcWork[5].w;
                                    temp_318 = fma(temp_309, temp_311, U_VolTexCalc.gVolTexCalcWork[6].x);
                                    temp_319 = fma(temp_309, temp_313, U_VolTexCalc.gVolTexCalcWork[6].y);
                                    temp_320 = fma(temp_309, temp_315, U_VolTexCalc.gVolTexCalcWork[6].z);
                                    temp_321 = fma(temp_309, temp_317, U_VolTexCalc.gVolTexCalcWork[6].w);
                                    temp_291 = false;
                                    temp_293 = temp_318;
                                    temp_295 = temp_319;
                                    temp_297 = temp_320;
                                    temp_299 = temp_321;
                                }
                                temp_322 = temp_291;
                                temp_323 = temp_293;
                                temp_324 = temp_295;
                                temp_325 = temp_297;
                                temp_326 = temp_299;
                                temp_327 = temp_322;
                                temp_328 = temp_323;
                                temp_329 = temp_324;
                                temp_330 = temp_325;
                                temp_331 = temp_326;
                                temp_157 = temp_323;
                                temp_160 = temp_324;
                                temp_163 = temp_325;
                                temp_154 = temp_326;
                                if (temp_322) {
                                    temp_301 = temp_302;
                                }
                                temp_332 = temp_301;
                                temp_333 = temp_332;
                                if (temp_322) {
                                    temp_334 = temp_332 + U_VolTexCalc.gVolTexCalcWork[2].y;
                                    temp_335 = temp_7 < temp_334;
                                    if (temp_335) {
                                        temp_327 = false;
                                    }
                                    temp_336 = temp_327;
                                    if (temp_335) {
                                        temp_328 = U_VolTexCalc.gVolTexCalcWork[5].x;
                                    }
                                    temp_337 = temp_328;
                                    temp_157 = temp_337;
                                    if (temp_335) {
                                        temp_329 = U_VolTexCalc.gVolTexCalcWork[5].y;
                                    }
                                    temp_338 = temp_329;
                                    temp_160 = temp_338;
                                    if (temp_335) {
                                        temp_330 = U_VolTexCalc.gVolTexCalcWork[5].z;
                                    }
                                    temp_339 = temp_330;
                                    temp_163 = temp_339;
                                    if (temp_335) {
                                        temp_331 = U_VolTexCalc.gVolTexCalcWork[5].w;
                                    }
                                    temp_340 = temp_331;
                                    temp_154 = temp_340;
                                    if (temp_336) {
                                        temp_333 = temp_334;
                                    }
                                    temp_341 = temp_333;
                                    if (temp_336) {
                                        temp_342 = temp_341 + U_VolTexCalc.gVolTexCalcWork[2].x;
                                        temp_343 = temp_7 < temp_342;
                                        if (temp_343) {
                                            temp_344 = 0. - temp_341;
                                            temp_345 = temp_344 + temp_342;
                                            temp_346 = 1. / temp_345;
                                            temp_347 = 0. - temp_341;
                                            temp_348 = temp_347 + temp_7;
                                            temp_349 = 0. - U_VolTexCalc.gVolTexCalcWork[5].x;
                                            temp_350 = temp_131 + temp_349;
                                            temp_351 = 0. - U_VolTexCalc.gVolTexCalcWork[5].y;
                                            temp_352 = temp_137 + temp_351;
                                            temp_353 = 0. - U_VolTexCalc.gVolTexCalcWork[5].z;
                                            temp_354 = temp_144 + temp_353;
                                            temp_355 = 0. - U_VolTexCalc.gVolTexCalcWork[5].w;
                                            temp_356 = temp_142 + temp_355;
                                            temp_357 = temp_348 * temp_346;
                                            temp_358 = fma(temp_350, temp_357, U_VolTexCalc.gVolTexCalcWork[5].x);
                                            temp_359 = fma(temp_352, temp_357, U_VolTexCalc.gVolTexCalcWork[5].y);
                                            temp_360 = fma(temp_354, temp_357, U_VolTexCalc.gVolTexCalcWork[5].z);
                                            temp_361 = fma(temp_356, temp_357, U_VolTexCalc.gVolTexCalcWork[5].w);
                                            temp_157 = temp_358;
                                            temp_160 = temp_359;
                                            temp_163 = temp_360;
                                            temp_154 = temp_361;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    temp_362 = temp_157;
    temp_363 = temp_160;
    temp_364 = temp_163;
    temp_365 = temp_154;
    temp_366 = texture(texShadow, vec2(temp_62, temp_63)).z;
    temp_367 = in_attr2.w;
    temp_368 = fma(U_Mate.gWrkFl4[2].z, 255., 0.5);
    temp_369 = trunc(temp_368);
    temp_370 = int(temp_369);
    temp_371 = float(temp_370);
    temp_372 = temp_371 + 0.5;
    temp_373 = fma(temp_367, 0.5, 0.5);
    temp_374 = temp_372 * 0.00390625;
    temp_375 = texture(gTToonDarkGrad, vec2(temp_373, temp_374)).xyz;
    temp_376 = temp_375.x;
    temp_377 = temp_375.y;
    temp_378 = temp_375.z;
    temp_379 = texture(gTToonGrad, vec2(temp_373, temp_374)).xyz;
    temp_380 = temp_379.x;
    temp_381 = temp_379.y;
    temp_382 = temp_379.z;
    temp_383 = in_attr2.x;
    temp_384 = in_attr2.y;
    temp_385 = 0. - U_Toon2.gToonParam[0].y;
    temp_386 = fma(temp_366, temp_385, temp_366);
    temp_387 = temp_386 + U_Toon2.gToonParam[0].y;
    temp_388 = in_attr2.z;
    temp_389 = temp_387 * U_LGT.gLgtPreCol[0].x;
    temp_390 = fma(temp_389, U_Toon2.gToonParam[0].z, temp_73);
    temp_391 = temp_387 * U_LGT.gLgtPreCol[0].y;
    temp_392 = temp_387 * U_LGT.gLgtPreCol[0].z;
    temp_393 = fma(temp_391, U_Toon2.gToonParam[0].z, temp_74);
    temp_394 = temp_390 + temp_383;
    temp_395 = fma(temp_392, U_Toon2.gToonParam[0].z, temp_75);
    temp_396 = temp_393 + temp_384;
    temp_397 = in_attr4.z;
    temp_398 = temp_394 * 0.299;
    temp_399 = temp_395 + temp_388;
    temp_400 = in_attr4.x;
    temp_401 = fma(temp_396, 0.587, temp_398);
    temp_402 = fma(temp_399, 0.114, temp_401);
    temp_403 = in_attr4.y;
    temp_404 = temp_402 + 0.00001;
    temp_405 = 1. / temp_404;
    temp_406 = temp_405 * U_LGT.gLgtPreDir[0].w;
    temp_407 = min(temp_406, 1.);
    temp_408 = 0. - U_Toon2.gToonParam[3].x;
    temp_409 = fma(temp_402, temp_407, temp_408);
    temp_410 = temp_400 * temp_362;
    temp_411 = 0. - temp_376;
    temp_412 = temp_411 + temp_380;
    temp_413 = log2(temp_410);
    temp_414 = 0. - temp_377;
    temp_415 = temp_414 + temp_381;
    temp_416 = in_attr4.w;
    temp_417 = temp_409 * U_Toon2.gToonParam[3].y;
    temp_418 = clamp(temp_417, 0., 1.);
    temp_419 = 0. - temp_378;
    temp_420 = temp_419 + temp_382;
    temp_421 = temp_403 * temp_363;
    temp_422 = temp_397 * temp_364;
    temp_423 = fma(temp_418, temp_412, temp_376);
    temp_424 = log2(temp_421);
    temp_425 = fma(temp_418, temp_415, temp_377);
    temp_426 = log2(temp_422);
    temp_427 = fma(temp_418, temp_420, temp_378);
    temp_428 = log2(temp_423);
    temp_429 = temp_413 * U_Static.gCDep.w;
    temp_430 = log2(temp_425);
    temp_431 = temp_424 * U_Static.gCDep.w;
    temp_432 = log2(temp_427);
    temp_433 = temp_426 * U_Static.gCDep.w;
    temp_434 = in_attr5.y;
    temp_435 = temp_428 * 2.2;
    temp_436 = temp_430 * 2.2;
    temp_437 = in_attr5.x;
    temp_438 = exp2(temp_429);
    temp_439 = temp_432 * 2.2;
    temp_440 = exp2(temp_435);
    temp_441 = exp2(temp_431);
    temp_442 = in_attr5.z;
    temp_443 = exp2(temp_436);
    temp_444 = temp_438 * temp_440;
    temp_445 = exp2(temp_433);
    temp_446 = temp_441 * temp_443;
    temp_447 = exp2(temp_439);
    temp_448 = temp_394 * temp_444;
    temp_449 = in_attr5.w;
    temp_450 = temp_396 * temp_446;
    temp_451 = temp_445 * temp_447;
    temp_452 = temp_407 * temp_448;
    temp_453 = temp_399 * temp_451;
    temp_454 = temp_407 * temp_450;
    temp_455 = 0. - temp_452;
    temp_456 = fma(temp_71, temp_455, temp_437);
    temp_457 = temp_407 * temp_453;
    temp_458 = temp_70 * temp_416;
    temp_459 = temp_71 * temp_452;
    temp_460 = 0. - temp_454;
    temp_461 = fma(temp_71, temp_460, temp_434);
    temp_462 = temp_71 * temp_454;
    temp_463 = 0. - temp_457;
    temp_464 = fma(temp_71, temp_463, temp_442);
    temp_465 = temp_71 * temp_457;
    temp_466 = fma(temp_449, temp_456, temp_459);
    temp_467 = temp_438 * temp_365;
    temp_468 = fma(temp_449, temp_461, temp_462);
    temp_469 = temp_441 * temp_365;
    temp_470 = temp_445 * temp_365;
    temp_471 = fma(temp_449, temp_464, temp_465);
    out_attr0.x = temp_466;
    out_attr0.y = temp_468;
    out_attr0.z = temp_471;
    out_attr0.w = temp_458;
    out_attr1.x = temp_467;
    out_attr1.y = temp_469;
    out_attr1.z = temp_470;
    out_attr1.w = temp_458;
    return;
}
