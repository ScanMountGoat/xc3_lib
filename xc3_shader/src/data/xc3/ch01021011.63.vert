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
    vec4 gLightShaft;
    vec4 gWetParam[2];
}U_Static;
layout(binding = 0, std430) buffer _U_Bone {
    uint data[];
}U_Bone;
layout(binding = 1, std430) buffer _U_OdB {
    uint data[];
}U_OdB;
layout(location = 0) in vec4 vPos;
layout(location = 1) in vec4 nWgtIdx;
layout(location = 2) in vec4 vTex0;
layout(location = 3) in vec4 vColor;
layout(location = 4) in vec4 vNormal;
layout(location = 5) in vec4 vTan;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
layout(location = 5) out vec4 out_attr5;
layout(location = 6) out vec4 out_attr6;
void main() {
    precise float temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise float temp_4;
    precise float temp_5;
    int temp_6;
    int temp_7;
    precise float temp_8;
    uint temp_9;
    int temp_10;
    int temp_11;
    int temp_12;
    precise float temp_13;
    precise float temp_14;
    int temp_15;
    precise float temp_16;
    precise float temp_17;
    uint temp_18;
    precise float temp_19;
    int temp_20;
    uint temp_21;
    precise float temp_22;
    int temp_23;
    uint temp_24;
    precise float temp_25;
    int temp_26;
    uint temp_27;
    precise float temp_28;
    int temp_29;
    precise float temp_30;
    precise float temp_31;
    precise float temp_32;
    precise float temp_33;
    uint temp_34;
    precise float temp_35;
    int temp_36;
    uint temp_37;
    precise float temp_38;
    int temp_39;
    uint temp_40;
    precise float temp_41;
    int temp_42;
    uint temp_43;
    precise float temp_44;
    precise float temp_45;
    uint temp_46;
    precise float temp_47;
    int temp_48;
    uint temp_49;
    precise float temp_50;
    int temp_51;
    uint temp_52;
    precise float temp_53;
    int temp_54;
    uint temp_55;
    precise float temp_56;
    precise float temp_57;
    uint temp_58;
    precise float temp_59;
    int temp_60;
    uint temp_61;
    precise float temp_62;
    int temp_63;
    uint temp_64;
    precise float temp_65;
    int temp_66;
    uint temp_67;
    precise float temp_68;
    uint temp_69;
    precise float temp_70;
    int temp_71;
    uint temp_72;
    precise float temp_73;
    int temp_74;
    uint temp_75;
    precise float temp_76;
    int temp_77;
    uint temp_78;
    precise float temp_79;
    precise float temp_80;
    uint temp_81;
    precise float temp_82;
    int temp_83;
    uint temp_84;
    precise float temp_85;
    int temp_86;
    uint temp_87;
    precise float temp_88;
    int temp_89;
    uint temp_90;
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
    precise float temp_111;
    precise float temp_112;
    precise float temp_113;
    precise float temp_114;
    precise float temp_115;
    precise float temp_116;
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
    precise float temp_165;
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
    gl_PointSize = 1.;
    gl_Position.x = 0.;
    gl_Position.y = 0.;
    gl_Position.z = 0.;
    gl_Position.w = 1.;
    temp_0 = nWgtIdx.x;
    temp_1 = vPos.x;
    temp_2 = vNormal.x;
    temp_3 = vTan.x;
    temp_4 = vPos.y;
    temp_5 = vNormal.y;
    temp_6 = floatBitsToInt(temp_0) & 65535;
    temp_7 = temp_6 * 48;
    temp_8 = vTan.y;
    temp_9 = floatBitsToUint(temp_0) >> 16;
    temp_10 = int(temp_9) * 48;
    temp_11 = temp_10 << 16;
    temp_12 = temp_11 + temp_7;
    temp_13 = vNormal.z;
    temp_14 = vTan.z;
    temp_15 = temp_12 + 16;
    temp_16 = vPos.z;
    temp_17 = vPos.w;
    temp_18 = uint(temp_12) >> 2;
    temp_19 = uintBitsToFloat(U_Bone.data[int(temp_18)]);
    temp_20 = temp_12 + 4;
    temp_21 = uint(temp_20) >> 2;
    temp_22 = uintBitsToFloat(U_Bone.data[int(temp_21)]);
    temp_23 = temp_12 + 8;
    temp_24 = uint(temp_23) >> 2;
    temp_25 = uintBitsToFloat(U_Bone.data[int(temp_24)]);
    temp_26 = temp_12 + 12;
    temp_27 = uint(temp_26) >> 2;
    temp_28 = uintBitsToFloat(U_Bone.data[int(temp_27)]);
    temp_29 = temp_12 + 32;
    temp_30 = vTan.w;
    temp_31 = vColor.x;
    temp_32 = vColor.y;
    temp_33 = vColor.z;
    temp_34 = uint(temp_15) >> 2;
    temp_35 = uintBitsToFloat(U_Bone.data[int(temp_34)]);
    temp_36 = temp_15 + 4;
    temp_37 = uint(temp_36) >> 2;
    temp_38 = uintBitsToFloat(U_Bone.data[int(temp_37)]);
    temp_39 = temp_15 + 8;
    temp_40 = uint(temp_39) >> 2;
    temp_41 = uintBitsToFloat(U_Bone.data[int(temp_40)]);
    temp_42 = temp_15 + 12;
    temp_43 = uint(temp_42) >> 2;
    temp_44 = uintBitsToFloat(U_Bone.data[int(temp_43)]);
    temp_45 = vColor.w;
    temp_46 = uint(temp_29) >> 2;
    temp_47 = uintBitsToFloat(U_Bone.data[int(temp_46)]);
    temp_48 = temp_29 + 4;
    temp_49 = uint(temp_48) >> 2;
    temp_50 = uintBitsToFloat(U_Bone.data[int(temp_49)]);
    temp_51 = temp_29 + 8;
    temp_52 = uint(temp_51) >> 2;
    temp_53 = uintBitsToFloat(U_Bone.data[int(temp_52)]);
    temp_54 = temp_29 + 12;
    temp_55 = uint(temp_54) >> 2;
    temp_56 = uintBitsToFloat(U_Bone.data[int(temp_55)]);
    temp_57 = vTex0.x;
    temp_58 = uint(temp_12) >> 2;
    temp_59 = uintBitsToFloat(U_OdB.data[int(temp_58)]);
    temp_60 = temp_12 + 4;
    temp_61 = uint(temp_60) >> 2;
    temp_62 = uintBitsToFloat(U_OdB.data[int(temp_61)]);
    temp_63 = temp_12 + 8;
    temp_64 = uint(temp_63) >> 2;
    temp_65 = uintBitsToFloat(U_OdB.data[int(temp_64)]);
    temp_66 = temp_12 + 12;
    temp_67 = uint(temp_66) >> 2;
    temp_68 = uintBitsToFloat(U_OdB.data[int(temp_67)]);
    temp_69 = uint(temp_15) >> 2;
    temp_70 = uintBitsToFloat(U_OdB.data[int(temp_69)]);
    temp_71 = temp_15 + 4;
    temp_72 = uint(temp_71) >> 2;
    temp_73 = uintBitsToFloat(U_OdB.data[int(temp_72)]);
    temp_74 = temp_15 + 8;
    temp_75 = uint(temp_74) >> 2;
    temp_76 = uintBitsToFloat(U_OdB.data[int(temp_75)]);
    temp_77 = temp_15 + 12;
    temp_78 = uint(temp_77) >> 2;
    temp_79 = uintBitsToFloat(U_OdB.data[int(temp_78)]);
    temp_80 = vTex0.y;
    temp_81 = uint(temp_29) >> 2;
    temp_82 = uintBitsToFloat(U_OdB.data[int(temp_81)]);
    temp_83 = temp_29 + 4;
    temp_84 = uint(temp_83) >> 2;
    temp_85 = uintBitsToFloat(U_OdB.data[int(temp_84)]);
    temp_86 = temp_29 + 8;
    temp_87 = uint(temp_86) >> 2;
    temp_88 = uintBitsToFloat(U_OdB.data[int(temp_87)]);
    temp_89 = temp_29 + 12;
    temp_90 = uint(temp_89) >> 2;
    temp_91 = uintBitsToFloat(U_OdB.data[int(temp_90)]);
    out_attr4.x = temp_31;
    out_attr4.y = temp_32;
    out_attr4.z = temp_33;
    out_attr4.w = temp_45;
    out_attr3.x = temp_57;
    out_attr3.y = temp_80;
    temp_92 = temp_19 * temp_1;
    temp_93 = temp_19 * temp_2;
    temp_94 = temp_19 * temp_3;
    temp_95 = fma(temp_22, temp_4, temp_92);
    temp_96 = temp_47 * temp_2;
    temp_97 = temp_35 * temp_2;
    temp_98 = temp_59 * temp_1;
    temp_99 = fma(temp_22, temp_5, temp_93);
    temp_100 = fma(temp_22, temp_8, temp_94);
    temp_101 = temp_35 * temp_1;
    temp_102 = fma(temp_62, temp_4, temp_98);
    temp_103 = temp_35 * temp_3;
    temp_104 = temp_47 * temp_3;
    temp_105 = temp_47 * temp_1;
    temp_106 = temp_70 * temp_1;
    temp_107 = temp_82 * temp_1;
    temp_108 = fma(temp_50, temp_5, temp_96);
    temp_109 = fma(temp_38, temp_5, temp_97);
    temp_110 = fma(temp_38, temp_4, temp_101);
    temp_111 = fma(temp_38, temp_8, temp_103);
    temp_112 = fma(temp_25, temp_13, temp_99);
    temp_113 = fma(temp_53, temp_13, temp_108);
    out_attr0.x = temp_112;
    temp_114 = fma(temp_41, temp_14, temp_111);
    out_attr0.z = temp_113;
    temp_115 = fma(temp_41, temp_13, temp_109);
    out_attr1.y = temp_114;
    temp_116 = fma(temp_50, temp_8, temp_104);
    out_attr0.y = temp_115;
    temp_117 = fma(temp_25, temp_16, temp_95);
    temp_118 = fma(temp_41, temp_16, temp_110);
    temp_119 = fma(temp_25, temp_14, temp_100);
    temp_120 = fma(temp_53, temp_14, temp_116);
    out_attr1.x = temp_119;
    temp_121 = fma(temp_44, temp_17, temp_118);
    out_attr1.z = temp_120;
    temp_122 = temp_115 * temp_119;
    temp_123 = temp_120 * temp_112;
    temp_124 = 0. - temp_122;
    temp_125 = fma(temp_114, temp_112, temp_124);
    temp_126 = fma(temp_50, temp_4, temp_105);
    temp_127 = fma(temp_28, temp_17, temp_117);
    temp_128 = temp_113 * temp_114;
    temp_129 = temp_125 * temp_30;
    temp_130 = fma(temp_73, temp_4, temp_106);
    out_attr2.z = temp_129;
    temp_131 = fma(temp_53, temp_16, temp_126);
    temp_132 = fma(temp_65, temp_16, temp_102);
    temp_133 = 0. - temp_128;
    temp_134 = fma(temp_120, temp_115, temp_133);
    temp_135 = 0. - temp_123;
    temp_136 = fma(temp_113, temp_119, temp_135);
    temp_137 = temp_127 * U_Static.gmProj[1].x;
    temp_138 = temp_127 * U_Static.gmProj[2].x;
    temp_139 = fma(temp_85, temp_4, temp_107);
    temp_140 = fma(temp_56, temp_17, temp_131);
    temp_141 = fma(temp_76, temp_16, temp_130);
    temp_142 = fma(temp_68, temp_17, temp_132);
    temp_143 = temp_134 * temp_30;
    temp_144 = fma(temp_121, U_Static.gmProj[2].y, temp_138);
    out_attr2.x = temp_143;
    temp_145 = temp_136 * temp_30;
    temp_146 = fma(temp_121, U_Static.gmProj[1].y, temp_137);
    out_attr2.y = temp_145;
    temp_147 = temp_127 * U_Static.gmProj[3].x;
    temp_148 = temp_127 * U_Static.gmProj[0].x;
    temp_149 = fma(temp_88, temp_16, temp_139);
    temp_150 = fma(temp_79, temp_17, temp_141);
    temp_151 = temp_142 * U_Static.gmProj[3].x;
    temp_152 = temp_142 * U_Static.gmProj[2].x;
    temp_153 = temp_142 * U_Static.gmProj[0].x;
    temp_154 = temp_142 * U_Static.gmProj[1].x;
    temp_155 = fma(temp_140, U_Static.gmProj[2].z, temp_144);
    temp_156 = fma(temp_140, U_Static.gmProj[1].z, temp_146);
    temp_157 = fma(temp_121, U_Static.gmProj[3].y, temp_147);
    temp_158 = fma(temp_121, U_Static.gmProj[0].y, temp_148);
    temp_159 = fma(temp_91, temp_17, temp_149);
    temp_160 = fma(temp_150, U_Static.gmProj[3].y, temp_151);
    temp_161 = fma(temp_150, U_Static.gmProj[2].y, temp_152);
    temp_162 = fma(temp_150, U_Static.gmProj[1].y, temp_154);
    temp_163 = fma(temp_150, U_Static.gmProj[0].y, temp_153);
    temp_164 = temp_155 + U_Static.gmProj[2].w;
    temp_165 = temp_156 + U_Static.gmProj[1].w;
    gl_Position.z = temp_164;
    temp_166 = fma(temp_140, U_Static.gmProj[3].z, temp_157);
    gl_Position.y = temp_165;
    temp_167 = fma(temp_140, U_Static.gmProj[0].z, temp_158);
    out_attr5.y = temp_165;
    temp_168 = fma(temp_159, U_Static.gmProj[3].z, temp_160);
    temp_169 = fma(temp_159, U_Static.gmProj[2].z, temp_161);
    temp_170 = fma(temp_159, U_Static.gmProj[1].z, temp_162);
    temp_171 = fma(temp_159, U_Static.gmProj[0].z, temp_163);
    temp_172 = 0. - U_Static.gCDep.y;
    temp_173 = temp_164 + temp_172;
    temp_174 = temp_166 + U_Static.gmProj[3].w;
    temp_175 = temp_167 + U_Static.gmProj[0].w;
    gl_Position.w = temp_174;
    temp_176 = temp_168 + U_Static.gmProj[3].w;
    out_attr5.w = temp_174;
    temp_177 = temp_169 + U_Static.gmProj[2].w;
    gl_Position.x = temp_175;
    temp_178 = temp_170 + U_Static.gmProj[1].w;
    out_attr5.x = temp_175;
    temp_179 = temp_171 + U_Static.gmProj[0].w;
    out_attr6.w = temp_176;
    temp_180 = temp_173 * U_Static.gCDep.z;
    out_attr6.z = temp_177;
    out_attr6.y = temp_178;
    out_attr6.x = temp_179;
    out_attr5.z = temp_180;
    return;
}
