[0000]   BLOCK (exit -> @02D2)
[0004]     AWAIT gameflag_252A
[0005]     GUARD active_actor == Scruter_Jo.talk (related 40)
[000A]     GUARD rec_0F4E == 3488
[000F]     ENDIF
[0010]     SAY "ME SCRUTER JO: SCANNING STRANGER...XRAY....STOP"  '[voice 1]
[0022]     SAY "ZONE ULTRAFORBIDDEN YOU NOT STAY HERE ... YOU GO AWAY ..."  '[voice 1]
[0040]     SAY "FIRST WARNING ..."  '[voice 1, skip 1]
[004E]     SETCHAR slot 4 = "scrut"
[0057]     IF-BLOCK (exit -> @00AC)
[005A]       GUARD rec_0740 > 1
[0061]       ENDIF
[0062]       SAY "Oh no ! Him again ... Commander , I don't feel happy about that SCRUT robot ..."
[008C]       SAY "We'll need to be smart , cunning , intelligent and clever ..."
    END
[00AC]     IF-BLOCK (exit -> @00D7)
[00AF]       GUARD rec_0740 == 2
[00B6]       ENDIF
[00B7]       SAY "Watch out , Commander ... Second try ... Don't blow it ..."
    END
[00D7]     IF-BLOCK (exit -> @0104)
[00DA]       GUARD rec_0740 == 3
[00E1]       ENDIF
[00E2]       SAY "I think I found the code , Commander... It's the number 9 ..."
    END
[0104]     SAY "You give identity code : word_65535 robyx code ulikan 69 exxos electret 666 9"  '[voice 1]
[012A]     IF-BLOCK (exit -> @01BE)
[012D]       GUARD concept == "exxos"
[0130]       ENDIF
[0131]       SAY "Yes ! Code be EXXOS. You be very Important Being ..."  '[voice 2]
[014F]       SAY "Nice work , Commmander , I've got to hand it to you ... How did you figure it out ?"
[017F]       SAY "Ol' Turkey Face Bob'll be proud of you when I tell him ..."
[01A1]       SAY "You did give code . You are the MASTER ..."  '[voice 1]
[01BD]       CLEAR concept_alt
    END
[01BE]     IF-BLOCK (exit -> @0227)
[01C1]       GUARD NOT concept == "exxos"
[01C5]       ENDIF
[01C6]       SAY "You unauthorized . Me exterminate you now ..."  '[voice 3]
[01DE]       SAY "Uh oh Commander . He's gonna zap the Orxx..."
[01F8]       SAY "Bye bye , unauthorized strangers ..."  '[voice 4]
[020C]       SAY "BAAANG!!!"  '[skip 3]
[0216]       LOADSTR "explo3.hnm"
[0223]       END PRESENTATION Scruter_Jo.talk
[0226]       CLEAR concept_alt
    END
[0227]     SAY "GIVE ME YOUR ORDERS , MASTER . I am yours ..."  '[voice 5]
[0245]     SAY "Commander , We found the code for programming him ..."
[0261]     SAY "Let's teleport him , Commander . I ought to take a good look at him ... word_65535 teleport refuse"
[0291]     IF-BLOCK (exit -> @02B3)
[0294]       GUARD concept == "teleport"
[0297]       ENDIF
[0298]       SAY "TELEPORT SCRUTER JO TO CRYOBOX"  '[skip 3]
[02AA]       rec_0722 = 65535
[02AF]       END PRESENTATION Scruter_Jo.talk
[02B2]       CLEAR concept_alt
    END
[02B3]     IF-BLOCK (exit -> @02D2)
[02B6]       GUARD concept == "refuse"
[02B9]       ENDIF
[02BA]       SAY "You're the commander , Commander ..."  '[skip 2]
[02CE]       CLEAR concept_alt
[02CF]       END PRESENTATION Scruter_Jo.talk
    END
  END
[02D2]   BLOCK (exit -> @0381)
[02D6]     AWAIT gameflag_274F
[02D7]     GUARD active_actor == Scruter_Jo.talk (related 40)
[02DC]     ENDIF
[02DD]     SAY "I've reprogrammed him , Commander ... And I've discovered something weird ..."
[02FD]     SAY "These SCRUT robots use a psychic structure based on CYBERSPACE ..."
[031B]     SAY "I've rigged a system so you can enter it , Commander... That way you can get some BIONIUM..."
[0347]     SAY "The energy source computers are wild about..."
[035D]     SAY "Ask SCRUTER JO a few questions , Commander . You'll understand ..."  '[skip 1]
[037D]     POKE [0x02D3] = 0
  END
[0381]   BLOCK (exit -> @06B9)
[0385]     AWAIT gameflag_274F
[0386]     GUARD active_actor == Scruter_Jo.talk (related 40)
[038B]     ENDIF
[038C]     SAY "Commander you go get BIONIUM in CYBERSPACE of SCRUTER JO..."  '[voice 19]
[03A8]     IF-BLOCK (exit -> @0549)
[03AB]       GUARD compris == 0
[03B2]       ENDIF
[03B3]       SAY "Me explain to you how get BIONIUM..."
[03C9]       SAY "You find BIOXX . Bioxx be small energy creatures ..."
[03E5]       SAY "You touch BIOXX with hand ."  '[voice 1]
[03F9]       SAY "Sounds like a piece of cake to me , Commander !!"
[0417]       SAY "If you touch BIOXX, you CAPTURE BIOXX on tip of your finger ..."  '[voice 2]
[0439]       SAY "Catch him on the tip of your finger !!! Sounds easy as pie , Commander ..."
[0461]       SAY "Then you can carry BIOXX to cybernetic MANTAS"  '[voice 3]
[0479]       SAY "You place BIOXX in belly of Manta..."  '[voice 5]
[048F]       SAY "BIOXX stay stuck to MANTAS..."  '[voice 4]
[04A1]       SAY "I'd love to see that ..."
[04B5]       SAY "Mantas change BIOXX into BIONIUM..."  '[voice 6]
[04C7]       SAY "More BIOXX you give to Mantas, more BIONIUM you get ..."  '[voice 5]
[04E5]       SAY "Yes !!! BIONIUM ... I can taste it already ..."
[0501]       SAY "To come back from CYBERSPACE , you touch BLUE BOX ...."  '[voice 4]
[051F]       SAY "You understand ?"  '[voice 6]
[052D]       SAY "We understand perfectly , Mister SCRUTER JO... Right , Commander?"
    END
[0549]     SAY "YOU go , Commander ..."  '[voice 20]
[055B]     SAY "Ahh! Me feel better ..."  '[voice 21]
[056D]     IF-BLOCK (exit -> @0613)
[0570]       GUARD vbio > 0
[0577]       ENDIF
[0578]       SAY "Good work ... You did succeed ..."  '[voice 2]
[058E]       SAY "You did get BIONIUM..."  '[voice 3]
[059E]       SAY "YES !!! Commander , remind me to tell you you're a champ ..."
[05C0]       SAY "This BIONIUM is extraordinary . My clock frequency's through the roof ..."
[05E0]       SAY "I feel even smarter ... I can feel I'll be a great help to you , Commander ..."  '[skip 1]
[060C]       compris = 1
    END
[0613]     IF-BLOCK (exit -> @069C)
[0616]       GUARD vbio == 0
[061D]       ENDIF
[061E]       SAY "Not good , friend ... You fail ..."  '[voice 4]
[0636]       SAY "Commander, you didn't understand the technique ..."
[064C]       SAY "I need BIONIUM, Commander . It makes me smarter ..."
[0668]       SAY "Ha! Ha! You need much much BIONIUM . Ha! Ha!..."  '[voice 4]
[0684]       SAY "Why don't you shut up , wiseguy !!!"
    END
[069C]     SAY "Bye bye , Commander . Me return to CYBERSPACE..."  '[voice 7, skip 1]
[06B6]     END PRESENTATION Scruter_Jo.talk
  END
[06B9]   BLOCK (exit -> @076C)
[06BD]     AWAIT presentation
[06BE]     GUARD active_actor == menu.talk (related 40)
[06C3]     GUARD rec_0332 == 65535
[06C8]     ENDIF
[06C9]     SAY ""IMPROVED MENU""
[06D5]     SAY "Today CHEF BRONKO has laid on for you :"
[06EF]     SAY "Tasty MURFFALO soup Bronko-style ."
[0701]     SAY "MURFFALO kidneys Bronko-style ."
[0711]     SAY "MURFFALO hamburger with Bar-B-Q recycled-oil dip ."
[0727]     SAY "Smooth MURFFALO-chip ice cream ."
[0739]     SAY "Recycled water"
[0745]     SAY "Chef Bronko says ... Burping's bad manners ! ..."
[075F]     SAY "stop"  '[skip 1]
[0769]     END PRESENTATION menu.talk
  END
[076C]   BLOCK (exit -> @081C)
[0770]     AWAIT presentation
[0771]     GUARD active_actor == menu.talk (related 40)
[0776]     GUARD NOT rec_0332 == 65535
[077C]     ENDIF
[077D]     SAY ""MENU""
[0787]     SAY "Today's fare :"
[0795]     SAY "PLASMA soup HONK-style ."
[07A5]     SAY "WRIGGLER belly in slobber sauce ."
[07B9]     SAY "Jellied URTIKAN with MURFFALO bone marrow ."
[07CF]     SAY "GLOK eye pie ."
[07DF]     SAY "Recycled water"
[07EB]     SAY "The chef says ... Don't eat with your mouth full ! ..."
[080B]     SAY "Stop"  '[skip 2]
[0815]     POKE [0x076D] = 0
[0819]     END PRESENTATION menu.talk
  END
[081C]   BLOCK (exit -> @08C6)
[0820]     AWAIT presentation
[0821]     GUARD active_actor == menu.talk (related 40)
[0826]     GUARD NOT rec_0332 == 65535
[082C]     ENDIF
[082D]     SAY ""MENU""
[0837]     SAY "Today's fare :"
[0845]     SAY "PLASMA soup HONK-style ."
[0855]     SAY "WRIGGLER snout stew ."
[0865]     SAY "URTIKAN seeds in MURFFALO venom ."
[0879]     SAY "GLOK juice dessert ."
[0889]     SAY "Recycled water"
[0895]     SAY "The chef says ... Don't talk with your mouth open ! ..."
[08B5]     SAY "stop"  '[skip 2]
[08BF]     POKE [0x081D] = 0
[08C3]     END PRESENTATION menu.talk
  END
[08C6]   BLOCK (exit -> @0972)
[08CA]     AWAIT presentation
[08CB]     GUARD NOT rec_0332 == 65535
[08D1]     GUARD active_actor == menu.talk (related 40)
[08D6]     ENDIF
[08D7]     SAY ""MENU""
[08E1]     SAY "Today's fare :"
[08EF]     SAY "PLASMA soup HONK-style ."
[08FF]     SAY "WRIGGLER feet in emulsive sauce ."
[0913]     SAY "URTIKAN leaves in MURFFALO sweat ."
[0927]     SAY "GLOK flake dessert ."
[0937]     SAY "Recycled water"
[0943]     SAY "The chef says ... Somebody didn't finish his wrigglers yesterday ..."
[0961]     SAY "stop"  '[skip 2]
[096B]     POKE [0x08C7] = 0
[096F]     END PRESENTATION menu.talk
  END
[0972]   BLOCK (exit -> @0A26)
[0976]     AWAIT presentation
[0977]     GUARD active_actor == menu.talk (related 40)
[097C]     GUARD NOT rec_0332 == 65535
[0982]     ENDIF
[0983]     SAY ""MENU""
[098D]     SAY "Today's fare :"
[099B]     SAY "HONK-style PLASMA soup ."
[09AB]     SAY "WRIGGLER brain , stewed in its own juice ."
[09C5]     SAY "URTIKAN trunk , stuffed with MURFFALO liver ."
[09DD]     SAY "GLOK dee-lite ."
[09EB]     SAY "Recycled water"
[09F7]     SAY "The chef says ... Plenty more in the kitchen ! ..."
[0A15]     SAY "stop"  '[skip 2]
[0A1F]     POKE [0x0973] = 0
[0A23]     END PRESENTATION menu.talk
  END
[0A26]   BLOCK (exit -> @0AE2)
[0A2A]     AWAIT presentation
[0A2B]     GUARD active_actor == menu.talk (related 40)
[0A30]     GUARD NOT rec_0332 == 65535
[0A36]     ENDIF
[0A37]     SAY ""IMPROVED MENU""
[0A43]     SAY "Today's fare :"
[0A51]     SAY "Soup of PLASMA HONK-style ."
[0A63]     SAY "WRIGGLER hearts in green blood coagulate ."
[0A79]     SAY "URTIKAN roots , deep fried in recycled oil ."
[0A93]     SAY "Candied GLOK tongue ."
[0AA3]     SAY "Recycled water"
[0AAF]     SAY "The chef says ... You eat what you are ! ..."
[0ACD]     SAY "stop"  '[skip 3]
[0AD7]     POKE [0x0A27] = 0
[0ADB]     POKE [0x0AE3] = 1
[0ADF]     END PRESENTATION menu.talk
  END
[0AE2]   GOTO @0AFF
[0AE6]   ENDIF
[0AE7]   POKE [0x076D] = 1
[0AEB]   POKE [0x081D] = 1
[0AEF]   POKE [0x08C7] = 1
[0AF3]   POKE [0x0973] = 1
[0AF7]   POKE [0x0A27] = 1
[0AFB]   POKE [0x0AE3] = 0
[0AFF]   BLOCK (exit -> @0B82)
[0B03]     AWAIT presentation
[0B04]     GUARD active_actor == Honk.talk (related 40)
[0B09]     ENDIF
[0B0A]     IF-BLOCK (exit -> @0B37)
[0B0D]       GUARD plus == 0
[0B14]       ENDIF
[0B15]       SAY "Commander , Cap'n Bob's a secretive buzzard ... He is hiding something ..."
    END
[0B37]     IF-BLOCK (exit -> @0B82)
[0B3A]       GUARD plus == 1
[0B41]       ENDIF
[0B42]       SAY "Commander, I hope you're gonna tell me what ol' Turkey face said when he unplugged me ..."
[0B6C]       SAY "It's a secret ... Right Commander ..."
    END
  END
[0B82]   BLOCK (exit -> @0DDF)
[0B86]     AWAIT presentation
[0B87]     GUARD active_actor == Honk.talk (related 40)
[0B8C]     ENDIF
[0B8D]     SAY ""  '[skip 2]
[0B95]     adieu = 0
[0B9C]     state[15] = 100
[0BA0]     IF-BLOCK (exit -> @0BC7)
[0BA3]       GUARD rec_0080 == 0
[0BAA]       ENDIF
[0BAB]       SAY "Commander , remember ol' Bob snoring in the Cryobox ..."
    END
[0BC7]     IF-BLOCK (exit -> @0DDF)
[0BCA]       GUARD rec_0722 == 65535
[0BCF]       ENDIF
[0BD0]       IF-BLOCK (exit -> @0C3D)
[0BD3]         GUARD vbio == 0
[0BDA]         ENDIF
[0BDB]         SAY "Commander , we have no BIONIUM... COMMANDER ... Please"
[0BF5]         SAY "I need that energy ..."
[0C07]         SAY "You have to go into Scruter Jo's CYBERSPACE ..."
[0C21]         SAY "Wake up Scruter_Jo, Commander . He's sleeping in the Cryobox..."
      END
[0C3D]       IF-BLOCK (exit -> @0CA4)
[0C40]         GUARD vbio == 1
[0C47]         ENDIF
[0C48]         SAY "We've only got one dose of BIONIUM left , Commander"
[0C64]         SAY "You should think about a trip to SCRUTER JO's CYBERSPACE..."
[0C80]         SAY "I'm not feeling too sure of myself ... Commander... I really need that energy..."
      END
[0CA4]       IF-BLOCK (exit -> @0CE9)
[0CA7]         GUARD vbio == 2
[0CAE]         ENDIF
[0CAF]         SAY "We've got two doses of BIONIUM left , Commander"
[0CC9]         SAY "It's about time you paid a visit to SCRUTER JO's CYBERSPACE ..."
      END
[0CE9]       IF-BLOCK (exit -> @0D0E)
[0CEC]         GUARD vbio == 3
[0CF3]         ENDIF
[0CF4]         SAY "We've got three doses of BIONIUM left , Commander"
      END
[0D0E]       IF-BLOCK (exit -> @0D33)
[0D11]         GUARD vbio == 4
[0D18]         ENDIF
[0D19]         SAY "We've got four doses of BIONIUM left , Commander"
      END
[0D33]       IF-BLOCK (exit -> @0D58)
[0D36]         GUARD vbio == 5
[0D3D]         ENDIF
[0D3E]         SAY "We've got five doses of BIONIUM left , Commander"
      END
[0D58]       IF-BLOCK (exit -> @0D7D)
[0D5B]         GUARD vbio == 6
[0D62]         ENDIF
[0D63]         SAY "We've got six doses of BIONIUM left , Commander"
      END
[0D7D]       IF-BLOCK (exit -> @0DA2)
[0D80]         GUARD vbio == 7
[0D87]         ENDIF
[0D88]         SAY "We've got seven doses of BIONIUM left , Commander"
      END
[0DA2]       IF-BLOCK (exit -> @0DDF)
[0DA5]         GUARD vbio == 8
[0DAC]         ENDIF
[0DAD]         SAY "We've got eight doses of BIONIUM left , Commander"
[0DC7]         SAY "You're the best of the best , Commander..."
      END
    END
  END
[0DDF]   BLOCK (exit -> @0E49)
[0DE3]     AWAIT presentation
[0DE4]     GUARD active_actor == Honk.talk (related 40)
[0DE9]     GUARD vbio > 2
[0DF0]     GUARD (rec_0B2C & 0x2) == 0
[0DF6]     GUARD rec_0590 > 0
[0DFD]     ENDIF
[0DFE]     SAY "Commander , I'm sure Izwalito knows other planetary coordinates ..."  '[skip 1]
[0E1A]     vbio -= 3
[0E21]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0E45]     POKE [0x0DE0] = 0
  END
[0E49]   BLOCK (exit -> @0EB3)
[0E4D]     AWAIT presentation
[0E4E]     GUARD active_actor == Honk.talk (related 40)
[0E53]     GUARD vbio > 2
[0E5A]     GUARD rec_0230 > 1
[0E61]     GUARD (rec_0EB0 & 0x2) == 0
[0E67]     ENDIF
[0E68]     SAY "Commander, Yoko likes Slimers ... Ask him for information ..."  '[skip 1]
[0E84]     vbio -= 3
[0E8B]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0EAF]     POKE [0x0E4A] = 0
  END
[0EB3]   BLOCK (exit -> @0F26)
[0EB7]     AWAIT presentation
[0EB8]     GUARD active_actor == Honk.talk (related 40)
[0EBD]     GUARD vbio > 2
[0EC4]     GUARD (rec_0EB0 & 0x2) != 0
[0EC9]     GUARD (rec_097C & 0x2) == 0
[0ECF]     GUARD rec_0470 > 0
[0ED6]     ENDIF
[0ED7]     SAY "Commander, Daddy Gluxx talked about SLIM GELATI... Ask him for more information..."  '[skip 1]
[0EF7]     vbio -= 3
[0EFE]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0F22]     POKE [0x0EB4] = 0
  END
[0F26]   BLOCK (exit -> @0F93)
[0F2A]     AWAIT presentation
[0F2B]     GUARD active_actor == Honk.talk (related 40)
[0F30]     GUARD vbio > 2
[0F37]     GUARD rec_0332 == 65535
[0F3C]     GUARD rec_0860 == 0
[0F43]     GUARD rec_0842 == 3848
[0F48]     GUARD (rec_09E8 & 0x2) != 0
[0F4D]     ENDIF
[0F4E]     SAY "Commander, get back to the planet Moskito..."  '[skip 1]
[0F64]     vbio -= 3
[0F6B]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0F8F]     POKE [0x0F27] = 0
  END
[0F93]   BLOCK (exit -> @0FF8)
[0F97]     AWAIT presentation
[0F98]     GUARD active_actor == Honk.talk (related 40)
[0F9D]     GUARD vbio > 2
[0FA4]     GUARD rec_01E8 == 0
[0FAB]     GUARD (rec_09E8 & 0x2) != 0
[0FB0]     ENDIF
[0FB1]     SAY "Commander, you haven't seen Izwalito in a while..."  '[skip 1]
[0FC9]     vbio -= 3
[0FD0]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[0FF4]     POKE [0x0F94] = 0
  END
[0FF8]   BLOCK (exit -> @1060)
[0FFC]     AWAIT presentation
[0FFD]     GUARD active_actor == Honk.talk (related 40)
[1002]     GUARD vbio > 2
[1009]     GUARD rec_06B0 > 2
[1010]     GUARD NOT rec_1030 == 1658
[1016]     ENDIF
[1017]     SAY "Commander, SCRUTER MAC on Mastachok loves perfume from VENUSIA..."  '[skip 1]
[1031]     vbio -= 3
[1038]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[105C]     POKE [0x0FF9] = 0
  END
[1060]   BLOCK (exit -> @10C8)
[1064]     AWAIT presentation
[1065]     GUARD active_actor == Honk.talk (related 40)
[106A]     GUARD vbio > 2
[1071]     GUARD PP1 == 1
[1078]     GUARD NOT rec_1018 == 40
[107E]     ENDIF
[107F]     SAY "Commander, on VENUSIA, Bug Deluxe has lots of creds..."  '[skip 1]
[1099]     vbio -= 3
[10A0]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[10C4]     POKE [0x1061] = 0
  END
[10C8]   BLOCK (exit -> @1130)
[10CC]     AWAIT presentation
[10CD]     GUARD active_actor == Honk.talk (related 40)
[10D2]     GUARD vbio > 2
[10D9]     GUARD (rec_09E8 & 0x2) == 0
[10DF]     GUARD rec_02A2 == 3932
[10E4]     ENDIF
[10E5]     SAY "Commander, don't forget poor Morning Oil on the Ark ..."  '[skip 1]
[1101]     vbio -= 3
[1108]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[112C]     POKE [0x10C9] = 0
  END
[1130]   BLOCK (exit -> @11A3)
[1134]     AWAIT presentation
[1135]     GUARD active_actor == Honk.talk (related 40)
[113A]     GUARD vbio > 2
[1141]     GUARD (rec_097C & 0x2) != 0
[1146]     GUARD eka == 0
[114D]     GUARD rec_11B0 == 1514
[1152]     GUARD rec_0620 > 0
[1159]     ENDIF
[115A]     SAY "Commander, talk to Doctor Otto Von Smile about TRANSPLANT..."  '[skip 1]
[1174]     vbio -= 3
[117B]     SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 1]
[119F]     POKE [0x1131] = 0
  END
[11A3]   BLOCK (exit -> @1347)
[11A7]     AWAIT presentation
[11A8]     GUARD active_actor == Honk.talk (related 40)
[11AD]     ENDIF
[11AE]     SAY "..."
[11B8]     SAY "What do you want Commander ? word_65535 talk remember bye_bye"
[11D6]     IF-BLOCK (exit -> @11E2)
[11D9]       GUARD concept == "remember"
[11DC]       ENDIF
[11DD]       POKE [0x1348] = 1
[11E1]       CLEAR concept_alt
    END
[11E2]     IF-BLOCK (exit -> @11EF)
[11E5]       GUARD concept == "talk"
[11E8]       ENDIF
[11E9]       rec_08B8 = 2902
[11EE]       CLEAR concept_alt
    END
[11EF]     IF-BLOCK (exit -> @121E)
[11F2]       GUARD concept == "bye_bye"
[11F5]       ENDIF
[11F6]       SAY "Service with a smile ! That's the story of my life , Commander ..."  '[skip 2]
[121A]       CLEAR concept_alt
[121B]       END PRESENTATION Honk.talk
    END
[121E]     IF-BLOCK (exit -> @12A5)
[1221]       GUARD scr > 5
[1228]       ENDIF
[1229]       SAY "CHEAT MODE..."  '[skip 1]
[1235]       rec_08B8 = 2929
[123A]       SAY "Choose your script, Commander word_65535 3 4 5"
[1254]       IF-BLOCK (exit -> @126F)
[1257]         GUARD concept == "3"
[125A]         ENDIF
[125B]         SAY "Script 3 selected..."  '[skip 3]
[1269]         RUN PROFILE 2
[126B]         CLEAR concept_alt
[126C]         END PRESENTATION Honk.talk
      END
[126F]       IF-BLOCK (exit -> @128A)
[1272]         GUARD concept == "4"
[1275]         ENDIF
[1276]         SAY "Script 4 selected..."  '[skip 3]
[1284]         RUN PROFILE 3
[1286]         CLEAR concept_alt
[1287]         END PRESENTATION Honk.talk
      END
[128A]       IF-BLOCK (exit -> @12A5)
[128D]         GUARD concept == "5"
[1290]         ENDIF
[1291]         SAY "Script 5 selected..."  '[skip 3]
[129F]         RUN PROFILE 4
[12A1]         CLEAR concept_alt
[12A2]         END PRESENTATION Honk.talk
      END
    END
[12A5]     IF-BLOCK (exit -> @12BF)
[12A8]       GUARD concept == "bye_bye"
[12AB]       ENDIF
[12AC]       SAY "Bye bye , Commander..."  '[skip 1]
[12BC]       END PRESENTATION Honk.talk
    END
[12BF]     IF-BLOCK (exit -> @12FE)
[12C2]       GUARD state[15] == 0
[12C4]       ENDIF
[12C5]       SAY "Pardon me ,Commander , but I have work to do ... I have to quadrify the multiplexers ... See you !"  '[skip 2]
[12F7]       state[15] = 65535
[12FB]       END PRESENTATION Honk.talk
    END
[12FE]     IF-BLOCK (exit -> @1347)
[1301]       GUARD rec_0590 > 2
[1308]       GUARD NOT rec_1048 == 65535
[130E]       GUARD NOT rec_1018 == 40
[1314]       GUARD ti == 1
[131B]       ENDIF
[131C]       SAY "Commander, I just forged a CRED ! Take a look in the Cryobox ..."  '[skip 1]
[1340]       OP_CD CD 94 05 04 10 28 00
    END
  END
[1347]   GOTO @1C4C
[134B]   START PRESENTATION Honk.talk (related 40)
[1350]   rec_0D36 |= 0x2
[1355]   ENDIF
[1356]   IF-BLOCK (exit -> @13BD)
[1359]     GUARD trak19 == 1
[1360]     ENDIF
[1361]     SAY "Well , we woke Cap'n Bob in his Cryobox . He told us a bunch of stuff and pointed out my deficiencies ..."
[1397]     SAY "Then we fearlessly entered a black hole and flew 500,000 years into the past ..."
  END
[13BD]   SAY "We got to a zone with heavy traffic buildup , where a SCRUTER invited us out ..."
[13E7]   SAY "You agreed with my evaluation of the situation and we bravely retreated ..."
[1409]   IF-BLOCK (exit -> @144A)
[140C]     GUARD (rec_0D36 & 0x2) != 0
[1411]     ENDIF
[1412]     SAY "I immediately detected the planet Corpo . You won't forget to mention that fact to Cap'n Bob , will you , Commander ? ..."
  END
[144A]   IF-BLOCK (exit -> @147D)
[144D]     GUARD trak2 == 1
[1454]     ENDIF
[1455]     SAY "We first met Izwalito on the planet Corpo . He was one hungry little guy ..."
  END
[147D]   IF-BLOCK (exit -> @14A4)
[1480]     GUARD trak2b == 1
[1487]     ENDIF
[1488]     SAY "He told us where to locate the planet MAGNUS ..."
  END
[14A4]   IF-BLOCK (exit -> @14F5)
[14A7]     GUARD trak3 == 1
[14AE]     ENDIF
[14AF]     SAY "We came back to see him and he asked us to get some frozen murffalo meat from the planet Moskito"
[14DF]     SAY "And he gave us a CRED ..."
  END
[14F5]   IF-BLOCK (exit -> @1554)
[14F8]     GUARD trak4 == 1
[14FF]     ENDIF
[1500]     SAY "You made an error of judgment , Commander , when you purchased a lump of murffalo on Venusia"
[152C]     SAY "That wasn't what Izwalito had requested , was it ? He experienced feelings of impatience ..."
  END
[1554]   IF-BLOCK (exit -> @158B)
[1557]     GUARD trak5 == 1
[155E]     ENDIF
[155F]     SAY "We got a call from BOSSANOVA , Izwalito's love partner . She was unhappy about the situation ..."
  END
[158B]   IF-BLOCK (exit -> @15E0)
[158E]     GUARD trak6 == 1
[1595]     ENDIF
[1596]     SAY "Then we bought three tons of murffalo from Bronko on the planet Moskito ..."
[15BA]     SAY "And we brought it back to Izwalito who gave it to his grateful people ..."
  END
[15E0]   IF-BLOCK (exit -> @1611)
[15E3]     GUARD C0 == 1
[15EA]     ENDIF
[15EB]     SAY "You may recall we received a mysterious message from far away in the galaxy ..."
  END
[1611]   IF-BLOCK (exit -> @1648)
[1614]     GUARD C0 == 2
[161B]     ENDIF
[161C]     SAY "I easily decoded the first word of the coded message from far away . The word was coded"
  END
[1648]   IF-BLOCK (exit -> @1691)
[164B]     GUARD C0 == 3
[1652]     ENDIF
[1653]     SAY "I quickly decoded the entire message from far away . It said CODED MESSAGE ... You will tell Cap'n Bob , won't you , Commander ? ..."
  END
[1691]   IF-BLOCK (exit -> @16F8)
[1694]     GUARD trak1 == 1
[169B]     ENDIF
[169C]     SAY "We got a pirate message from another player who hooked up to your computer , remember ? ..."
[16C8]     SAY "That other player was impressively talented ... He must have used a neural net to link up with us ..."
  END
[16F8]   IF-BLOCK (exit -> @1765)
[16FB]     GUARD trak7 == 1
[1702]     ENDIF
[1703]     SAY "Izwalito arranged for us to meet Hom , a buddy of his , in the village of HITA ..."
[1731]     SAY "Hom is a fun person , but strictly no longer at this address mindwise . The poor guy's brain is tubular ..."
  END
[1765]   IF-BLOCK (exit -> @1792)
[1768]     GUARD trak8 == 1
[176F]     ENDIF
[1770]     SAY "We teleported Bronko into the cryobox and he gives the planet Ekatomb ..."
  END
[1792]   IF-BLOCK (exit -> @17E9)
[1795]     GUARD trak9 == 1
[179C]     ENDIF
[179D]     SAY "Ha! Ha! I chuckled when I recall that poor Croolis who replaced Bronko as murffalo sales-being on Moskito ..."
[17CB]     SAY "He's called EMASCULATOR , EVISCERATOR's sworn enemy. What a character ..."
  END
[17E9]   IF-BLOCK (exit -> @182C)
[17EC]     GUARD trak10 == 1
[17F3]     ENDIF
[17F4]     SAY "We visited Migrator at the airport on Moskito"
[180C]     SAY "He talked about his music . We had a fine time ..."
  END
[182C]   IF-BLOCK (exit -> @18A7)
[182F]     GUARD trak11 == 1
[1836]     ENDIF
[1837]     SAY "We located TINA BURNER who was singing at the PURPLE HAZE on the planet Eden ..."
[185F]     SAY "We teleported her to Migrator on Moskito . He was overjoyed ..."
[187F]     SAY "And you lost your chance of finding true love , Commander ... Hee! Hee! Hee! ..."
  END
[18A7]   IF-BLOCK (exit -> @18F4)
[18AA]     GUARD trak12 == 1
[18B1]     ENDIF
[18B2]     SAY "A little later , Migrator announced his marriage to the luscious Tina . Poor guy ..."
[18DA]     SAY "He asked to give him the wedding ring ..."
  END
[18F4]   IF-BLOCK (exit -> @1961)
[18F7]     GUARD trak13 == 1
[18FE]     ENDIF
[18FF]     SAY "We came across Morning Oil on planet Magnus . Boy , was he in a lousy condition ..."
[192B]     SAY "I instantaneously noticed how flat his batteries were ... That's something you'll mention to Cap'n Bob , won't you , Commander ? ..."
  END
[1961]   IF-BLOCK (exit -> @19CE)
[1964]     GUARD trak14 == 1
[196B]     ENDIF
[196C]     SAY "We bought some batteries for poor old Morning Oil at VENUSIA supramarket ..."
[198E]     SAY "We put the new batteries in Morning Oil , and he started working again ..."
[19B4]     SAY "So we teleported him aboard to study him ..."
  END
[19CE]   IF-BLOCK (exit -> @19FF)
[19D1]     GUARD trak15 == 1
[19D8]     ENDIF
[19D9]     SAY "I was obliged to switch Morning Oil off , to check out his memory ..."
  END
[19FF]   IF-BLOCK (exit -> @1A36)
[1A02]     GUARD trak16 == 1
[1A09]     ENDIF
[1A0A]     SAY "I speedily discovered in Morning Oil's memory that the story about the Croolis war treasure was true ..."
  END
[1A36]   IF-BLOCK (exit -> @1A7B)
[1A39]     GUARD trak17 == 1
[1A40]     ENDIF
[1A41]     SAY "You interrogated Morning Oil and forced him to reveal the existence of the planet MASTACHOK , where his ex-master EVISCERATOR was to be found ."
  END
[1A7B]   IF-BLOCK (exit -> @1AD0)
[1A7E]     GUARD trak18 == 1
[1A85]     ENDIF
[1A86]     SAY "Then , uh , then we got a message from Hektor's wife . Hektor piloted that drone you destroyed , remember ? ..."
[1ABC]     SAY "That was a painful moment ..."
  END
[1AD0]   IF-BLOCK (exit -> @1AF5)
[1AD3]     GUARD trak20 == 1
[1ADA]     ENDIF
[1ADB]     SAY "Morning Oil told us about BIONIUM and ODDLAND CYBERSPACE"
  END
[1AF5]   IF-BLOCK (exit -> @1B16)
[1AF8]     GUARD trak21 == 1
[1AFF]     ENDIF
[1B00]     SAY "We met Yoko on the planet RONDO"
  END
[1B16]   IF-BLOCK (exit -> @1B3B)
[1B19]     GUARD trak22 == 1
[1B20]     ENDIF
[1B21]     SAY "Mister Maxxon explained about his telescope with no lens"
  END
[1B3B]   IF-BLOCK (exit -> @1B62)
[1B3E]     GUARD trak24 == 1
[1B45]     ENDIF
[1B46]     SAY "We ran into the GLUXX family on the planet Ekatomb"
  END
[1B62]   IF-BLOCK (exit -> @1B91)
[1B65]     GUARD trak25 == 1
[1B6C]     ENDIF
[1B6D]     SAY "Father GLUXX mentioned the planet Erazor where doctor Otto Von Smile has his clinic"
  END
[1B91]   IF-BLOCK (exit -> @1BB4)
[1B94]     GUARD trak26 == 1
[1B9B]     ENDIF
[1B9C]     SAY "We encountered the unutterably horrible doctor on Erazor"
  END
[1BB4]   IF-BLOCK (exit -> @1BE9)
[1BB7]     GUARD trak27 == 1
[1BBE]     ENDIF
[1BBF]     SAY "He gave us a telescope lens and we gave him the Gluxx's address on the planet Ekatomb"
  END
[1BE9]   IF-BLOCK (exit -> @1C2C)
[1BEC]     GUARD trak23 == 1
[1BF3]     ENDIF
[1BF4]     SAY "We returned to the planet Rondo"
[1C08]     SAY "We delivered Otto Von Smile's lens to Maxxon who seemed less than happy ..."
  END
[1C2C]   SAY "You need anything else ? Just say the word ..."  '[skip 1]
[1C48]   POKE [0x1348] = 0
[1C4C]   BLOCK (exit -> @1D73)
[1C50]     AWAIT gameflag_274F
[1C51]     GUARD active_actor == Bob_Morlock.talk (related 40)
[1C56]     GUARD BO == 0
[1C5D]     ENDIF
[1C5E]     SAY "HONK ! You worthless heap of wires ... Are you working ?"  '[voice 2]
[1C7E]     SAY "Yes sir, Cap'n Bob sir ! ... Just getting the multiplexers toned up ..."
[1CA2]     SAY "What do you want to know , Commander ?"  '[voice 3, skip 1]
[1CBC]     state[1] = 50
[1CC0]     IF-BLOCK (exit -> @1D14)
[1CC3]       GUARD state[1] == 0
[1CC5]       ENDIF
[1CC6]       SAY "I feel like a dog I micro-waved by mistake one day ... I have to cryonize ..."  '[voice 6]
[1CF0]       SAY "Ahhhh !!!"  '[voice 3]
[1CFC]       SAY "stop"  '[skip 3]
[1D06]       BO = 2
[1D0D]       state[1] = 65535
[1D11]       END PRESENTATION Bob_Morlock.talk
    END
[1D14]     SAY "I feel weak , Commander ... Let me sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[1D34]     adieu = 1
[1D3B]     IF-BLOCK (exit -> @1D73)
[1D3E]       GUARD adieu == 1
[1D45]       ENDIF
[1D46]       SAY "Ah, sleep ..."
[1D54]       SAY "stop"  '[skip 4]
[1D5E]       adieu = 0
[1D65]       BO = 2
[1D6C]       state[1] = 65535
[1D70]       END PRESENTATION Bob_Morlock.talk
    END
  END
[1D73]   BLOCK (exit -> @1F71)
[1D77]     AWAIT gameflag_274F
[1D78]     GUARD active_actor == Bob_Morlock.talk (related 40)
[1D7D]     GUARD BO == 2
[1D84]     ENDIF
[1D85]     SAY "My bones feel like they've been through a shredder ... HONK! Miserable machine ! Give me a shot of ASPIROX !!!"  '[voice 4, skip 1]
[1DB7]     adieu = 0
[1DBE]     SAY "Yes sir, Cap'n Bob . I'm locating the seringe even as I utter these words ..."
[1DE6]     SAY "Be careful ... Find a spot with some flesh on it . The last time , you stuck the needle straight into a bone !!!"  '[voice 5]
[1E20]     SAY "Yes ! Yes ! Cap'n Bob , I'll be careful , I'll shoot another buttock, I'll choose the third one ..."
[1E52]     SAY "I'm listening , Commander ..."  '[voice 3, skip 1]
[1E64]     state[1] = 60
[1E68]     IF-BLOCK (exit -> @1F0A)
[1E6B]       GUARD state[1] == 0
[1E6D]       ENDIF
[1E6E]       SAY "AAARGH !!! HONK YOU STABBED ME IN THE BONE AGAIN ... I told you not to do that ..."  '[voice 5]
[1E9C]       SAY "Sorry about that, Cap'n Bob . The needle slipped ..."
[1EB8]       SAY "I feel like the frog who stood up to the truck , Commander ! I have to cryonize ..."  '[voice 6]
[1EE6]       SAY "Ahhhh !!!"  '[voice 3]
[1EF2]       SAY "stop"  '[skip 3]
[1EFC]       BO = 3
[1F03]       state[1] = 65535
[1F07]       END PRESENTATION Bob_Morlock.talk
    END
[1F0A]     SAY "I feel awful , Commander ... You're tiring me out ... Let me sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[1F34]     adieu = 1
[1F3B]     IF-BLOCK (exit -> @1F71)
[1F3E]       GUARD adieu == 1
[1F45]       ENDIF
[1F46]       SAY "Sleep ..."
[1F52]       SAY "stop"  '[skip 4]
[1F5C]       adieu = 0
[1F63]       BO = 3
[1F6A]       state[1] = 65535
[1F6E]       END PRESENTATION Bob_Morlock.talk
    END
  END
[1F71]   BLOCK (exit -> @212C)
[1F75]     AWAIT gameflag_274F
[1F76]     GUARD active_actor == Bob_Morlock.talk (related 40)
[1F7B]     GUARD BO == 3
[1F82]     ENDIF
[1F83]     SAY "I'm thirsty ... HONK! You reject fridge component ! Give me a glass of recycled water !!!"  '[voice 4]
[1FAD]     SAY "Yes sir, Cap'n Bob ! Just locating a clean glass , beaker , mug , cup or other drinking vessel ..."
[1FDF]     SAY "Be careful ... I said recycled water , not liquid coolant like the last time !!!"  '[voice 5]
[2007]     SAY "Trust me , Cap'n Bob ! I've learned My lesson . Yes sir ! I'm going to taste it to make sure ..."
[203D]     SAY "Don't do that , you moronic box of longwinded incompetence ! You'll short-circuit !"  '[voice 3]
[2061]     SAY "I'm listening, Commander ..."  '[voice 3]
[2071]     state[1] = 120
[2075]     IF-BLOCK (exit -> @20B9)
[2078]       GUARD state[1] == 0
[207A]       ENDIF
[207B]       SAY "I'm too hot ... I need to cryonize ..."  '[voice 6]
[2095]       SAY "AAAAAAhhhh !!!"  '[voice 3]
[20A1]       SAY "stop"  '[skip 3]
[20AB]       BO = 4
[20B2]       state[1] = 65535
[20B6]       END PRESENTATION Bob_Morlock.talk
    END
[20B9]     SAY "I feel weak as that kitten I accidentally converted to anti-matter , Commander ... Let me get some sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[20ED]     adieu = 1
[20F4]     IF-BLOCK (exit -> @212C)
[20F7]       GUARD adieu == 1
[20FE]       ENDIF
[20FF]       SAY "Ah, sleep ..."
[210D]       SAY "stop"  '[skip 4]
[2117]       adieu = 0
[211E]       BO = 4
[2125]       state[1] = 65535
[2129]       END PRESENTATION Bob_Morlock.talk
    END
  END
[212C]   BLOCK (exit -> @22DA)
[2130]     AWAIT gameflag_274F
[2131]     GUARD active_actor == Bob_Morlock.talk (related 40)
[2136]     GUARD BO == 4
[213D]     ENDIF
[213E]     IF-BLOCK (exit -> @221B)
[2141]       GUARD rec_0080 == 6
[2148]       ENDIF
[2149]       SAY "I'm hungry ... HONK! You no-good collection of second-hand wiring ! Where's the menu !!!"  '[voice 4]
[216F]       SAY "Coming right up , Cap'n Bob ! Your dinner is heating up in the engine room ..."
[2199]       SAY "Be careful this time ... Hot food does not have to be curled up, black and smoking !!!"  '[voice 5]
[21C5]       SAY "Not smoking... You got it , Cap'n Bob , sir ... Would you care to browse through the MENU ?"
[21F5]       SAY "WHAT ?? GLOK AGAIN !!! I'd like to see you curled up and smoking ..."  '[voice 6]
    END
[221B]     SAY "What do you want to know Commander ..."  '[voice 6, skip 1]
[2233]     state[1] = 90
[2237]     IF-BLOCK (exit -> @226C)
[223A]       GUARD state[1] == 0
[223C]       ENDIF
[223D]       SAY "I need to cryonize ..."  '[voice 6]
[224F]       SAY "AAAAAAAAAAAAhhhh !!!"  '[voice 3]
[225B]       SAY "stop"  '[skip 2]
[2265]       state[1] = 65535
[2269]       END PRESENTATION Bob_Morlock.talk
    END
[226C]     SAY "I feel weak as that lamb I inadvertently dropped in my shark-pool one day , Commander ... Let me sleep ... word_65535 bye_bye"  '[voice 6, skip 1]
[22A2]     adieu = 1
[22A9]     IF-BLOCK (exit -> @22DA)
[22AC]       GUARD adieu == 1
[22B3]       ENDIF
[22B4]       SAY "Ah, sleep ..."
[22C2]       SAY "stop"  '[skip 3]
[22CC]       adieu = 0
[22D3]       state[1] = 65535
[22D7]       END PRESENTATION Bob_Morlock.talk
    END
  END
[22DA]   BLOCK (exit -> @23D2)
[22DE]     AWAIT gameflag_274F
[22DF]     GUARD active_actor == Bob_Morlock.talk (related 40)
[22E4]     GUARD emplo == 1
[22EB]     GUARD emp == 0
[22F2]     ENDIF
[22F3]     SAY "While you're awake, Cap'n Bob , sir ... I have a request to make ..."
[2319]     SAY "Olga and me would like a raise . A few megawatts ..."
[2339]     SAY "WHAT ... SPEAK LOUDER ... I'M GETTING DEAFER AND DEAFER ..."  '[voice 5]
[2357]     SAY "WE WANT MORE MEGAWATTS !!!"
[2369]     SAY "I feel like that canary I mistook for an oven-ready turkey , Commander . Haaa !!! My heart ...."  '[voice 4]
[2397]     SAY "Heart !! ... He means his power generator ... Those guys ... Just mention a couple of lousy megawatts !"  '[skip 2]
[23C7]     emp = 1
[23CE]     POKE [0x22DB] = 0
  END
[23D2]   BLOCK (exit -> @25DE)
[23D6]     AWAIT gameflag_274F
[23D7]     GUARD active_actor == Bob_Morlock.talk (related 40)
[23DC]     GUARD reve == 1
[23E3]     GUARD revelat == 0
[23EA]     ENDIF
[23EB]     SAY "You want to know an unbearable truth , Commander ?"  '[voice 7]
[2407]     SAY "HONK! Switch off for ten seconds !!!"  '[voice 6]
[241D]     SAY "But , Cap'n Bob !! ..."
[2431]     SAY "YOU HEARD ME ... SWITCH YOURSELF OFF !!!"  '[voice 5]
[2449]     SAY "Yes sir , Cap'n Bob . If you say so ..."
[2467]     SAY "KRUIIIIK !!! AAAaaaaaaaaaaaaaaaaaaaaa !!!"
[2477]     SAY "COMMANDER, YOU ARE ME ..."  '[voice 5]
[2489]     SAY "WE ARE THE SAME PERSON AT TWO DIFFERENT AGES ..."  '[voice 6]
[24A5]     SAY "YOU ARE MUCH MORE THAN MY SON ..."  '[voice 4]
[24BD]     SAY "We are the same person ... I am the first being to have created itself"  '[voice 5]
[24E3]     SAY "And who , thanks to space-time distortion , can watch itself re-live : YOU ARE BOB , COMMANDER ..."  '[voice 4]
[2511]     SAY "I am what you will become hundreds of thousands of years from now ..."  '[voice 2]
[2535]     SAY "OK Honk , you can switch on now ..."  '[voice 6]
[254F]     SAY "KROIIIIkkk !!! -&KRUIIIIkkk !!! You guys should know I have a right to stay informed . That's what being ONBOARD COMPUTER is all about , right ?"
[258D]     SAY "Was Olga switched off too ???"  '[skip 1]
[25A1]     plus = 1
[25A8]     SAY "Stop snivelling and get to work , you jumped up little fuse box ..."  '[voice 5, skip 3]
[25CC]     revelat = 1
[25D3]     reve = 0
[25DA]     POKE [0x23D3] = 0
  END
[25DE]   BLOCK (exit -> @272F)
[25E2]     AWAIT gameflag_274F
[25E3]     GUARD vol == 1
[25EA]     GUARD active_actor == Bob_Morlock.talk (related 40)
[25EF]     ENDIF
[25F0]     SAY "Cap'n Bob, I have to tell you the Commander has behaved very badly ..."
[2614]     SAY "He stole money from a defenceless Izwal of planet Corpo and then he spent it ..."
[263C]     SAY "I'm sorry to say it's true ..."
[2652]     SAY "Miserable machine !!! WHO DO YOU THINK YOU ARE ? He did whatever the situation demanded ..."  '[voice 5]
[267C]     SAY "True , Commander ? word_65535 yes no"  '[voice 4]
[2694]     IF-BLOCK (exit -> @26CA)
[2697]       GUARD concept == "yes"
[269A]       ENDIF
[269B]       SAY "Well ... If I remember , there wasn't a lot of choices ..."  '[voice 4]
[26BD]       SAY "Exactly ..."  '[skip 1]
[26C9]       CLEAR concept_alt
    END
[26CA]     IF-BLOCK (exit -> @272F)
[26CD]       GUARD concept == "no"
[26D0]       ENDIF
[26D1]       SAY "You see , you pathetic excuse for a machine : YOU'RE A LIAR !!! I knew it all along ..."  '[voice 5]
[2701]       SAY "But Cap'n Bob ... I swear it's true ..."
[271B]       SAY "SILENCE !!!"  '[voice 2, skip 2]
[2727]       vol = 0
[272E]       CLEAR concept_alt
    END
  END
[272F]   BLOCK (exit -> @2744)
[2733]     ENDIF
[2734]     state[3] = 10
[2738]     state[4] = 200
[273C]     OP_B7 B7 E0 06 02
[2740]     POKE [0x2730] = 0
  END
[2744]   BLOCK (exit -> @2758)
[2748]     GUARD state[3] == 0
[274A]     ENDIF
[274B]     OP_C3 C3 FC 06 28 00
[2750]     POKE [0x2759] = 1
[2754]     POKE [0x2745] = 0
  END
[2758]   GOTO @27CF
[275C]   AWAIT presentation
[275D]   START PRESENTATION Scruter_K.talk (related 40)
[2762]   ENDIF
[2763]   SAY ""  '[skip 1]
[276B]   SS = 1
[2772]   SAY ""  '[skip 1]
[277A]   SS = 2
[2781]   SAY ""  '[skip 1]
[2789]   SS = 3
[2790]   SAY ""  '[skip 1]
[2798]   SS = 4
[279F]   SAY ""  '[skip 1]
[27A7]   SS = 5
[27AE]   IF-BLOCK (exit -> @27BC)
[27B1]     GUARD SS == 0
[27B8]     ENDIF
[27B9]     GOTO @2763
  END
[27BC]   IF-BLOCK (exit -> @27CF)
[27BF]     GUARD SS > 0
[27C6]     ENDIF
[27C7]     POKE [0x2759] = 0
[27CB]     POKE [0x27D0] = 1
  END
[27CF]   GOTO @2DBE
[27D3]   AWAIT presentation
[27D4]   START PRESENTATION Scruter_K.talk (related 40)
[27D9]   ENDIF
[27DA]   IF-BLOCK (exit -> @2904)
[27DD]     GUARD SS == 1
[27E4]     ENDIF
[27E5]     SAY "MESSAGE RADIO:"
[27F1]     SAY "Calling garbage scow :"
[2801]     SAY "This is SCRUT robot "HYDRAULIC JACK OF TITANIUM ALLOY", squadron commander ..."
[2821]     SAY "You are in a forbidden zone ... THIS IS A WARNING ..."
[2841]     SAY "YOU ARE SCHEDULED FOR VAPORIZATION IN TWO MINUTES , TEN SECONDS ..."
[2861]     SAY "MAKE THAT NINE SECONDS ..."
[2873]     SAY "EIGHT SECONDS ..."
[2881]     SAY "SEVEN ..."
[288D]     SAY "Get that garbage wagon out of here ! Scram ! Otherwise my hi-powered laser weapons will spell death !!!..."
[28BB]     SAY "REPORT FROM THE ARK'S ONBOARD COMPUTER FACILITY , HONK :"
[28D7]     SAY "Commander , He means it ... the whole zone's filled with guys like him : RED ALERT"
[2901]     GOTO @2D40
  END
[2904]   IF-BLOCK (exit -> @2A14)
[2907]     GUARD SS == 2
[290E]     ENDIF
[290F]     SAY "MESSAGE RADIO:"
[291B]     SAY "LISTEN UP"
[2927]     SAY "This is SCRUT robot "NONSTICK HEART" , area sub-director , calling garbage can ..."
[294B]     SAY "THIS IS A WARNING ... YOU ARE IN A FORBIDDEN ZONE"
[2969]     SAY "PREPARE FOR PAINFUL OBLITERIZATION IN TEN SECONDS ..."
[2981]     SAY "NINE SECONDS ..."
[298F]     SAY "TWELVE SECONDS ..."
[299D]     SAY "UH , SORRY ABOUT THAT ..."
[29B1]     SAY "EIGHT ..."
[29BD]     SAY "LEAVE NOW OR MEET ROSEBUD ? MY LASER-PULPER"
[29D5]     SAY "Honk reports in at this time :"
[29EB]     SAY "I advise we comply fast , Commander. He's liable to skip four and three ..."
[2A11]     GOTO @2D40
  END
[2A14]   IF-BLOCK (exit -> @2AF4)
[2A17]     GUARD SS == 3
[2A1E]     ENDIF
[2A1F]     SAY "MESSAGE RADIO:"
[2A2B]     SAY "ATTENTION ..."
[2A37]     SAY "THIS IS SCRUT TROOPER "TUNG STEN" , TO UNKNOWN SHIP-TYPE THING ..."
[2A57]     SAY "I HAVE THE FOLLOWING WARNING : THIS SECTOR IS FORBIDDEN TO Y-O-U !!!"
[2A79]     SAY "WE WILL MANGLE YOUR BODY ORGANS , MELT DOWN YOUR MELTABLE PARTS , AND ALSO VITRIFY YOU IF YOU DON'T GO AWAY"
[2AAD]     SAY "THREE SECONDS ..."
[2ABB]     SAY "TWO SECONDS ..."
[2AC9]     SAY "Honk reports , in the nick of time :"
[2AE3]     SAY "Red Alert !"
[2AF1]     GOTO @2D40
  END
[2AF4]   IF-BLOCK (exit -> @2C2C)
[2AF7]     GUARD SS == 4
[2AFE]     ENDIF
[2AFF]     SAY "MESSAGE RADIO:"
[2B0B]     SAY "OKAY WHOEVER YOU ARE"
[2B1B]     SAY "This is low-ranking SCRUT robot "STAINLESS STEEL CASING" , CALLING FLYING JUNK HEAP ..."
[2B3F]     SAY "MOVE OUT OF THIS FORBIDDEN SECTOR ... AND MAKE IT SNAPPY , BUDDY BOY !!!"
[2B65]     SAY "Otherwise you get STOMPED ON ..."
[2B79]     SAY "YOU HAVE TEN SECONDS BEFORE ANNIHILATION SETS IN ..."
[2B93]     SAY "NINE SECONDS ..."
[2BA1]     SAY "TWELVE SECONDS ..."
[2BAF]     SAY "OKAY OKAY , WISE GUY !"
[2BC3]     SAY "YOU DO THE COUNTING ..."
[2BD5]     SAY "CRUIIIIK!"
[2BDF]     SAY "Report from Honk , onboard computing entity :"
[2BF7]     SAY "It's true he's no shining beacon of brainpower , Commander . And yet I feel moved by his obvious sincerity ..."
[2C29]     GOTO @2D40
  END
[2C2C]   IF-BLOCK (exit -> @2D40)
[2C2F]     GUARD SS == 5
[2C36]     ENDIF
[2C37]     SAY "MESSAGE RADIO:"
[2C43]     SAY "LISTEN UP"
[2C4F]     SAY "THIS IS SCRUT ROBOT "TENSILE SHEET METAL" , AREA CHIEF , TO UNKNOWN VESSEL ..."
[2C75]     SAY "THIS IS A PL7K stroke 38B WARNING : YOU HAVE TWO MINUTES TO EVACUATE THE SECTOR"
[2C9D]     SAY "FAILURE TO COMPLY WITH ABOVE PL7K stroke 38B WARNING WILL RESULT IN A 1010 ..."
[2CC3]     SAY "YOURS SINCERELY ,"
[2CD1]     SAY "SIGNED : SCRUTER "TENSILE SHEET METAL" , AREA CHIEF WITH ADDED LASER POWER"
[2CF3]     SAY "Heeeere's Honky ! ..."
[2D03]     SAY "That stuff about 1010 ? That's their way of saying meet your maker . There's a big bunch of them SCRUT ships around here ..."
[2D3D]     GOTO @2D40
  END
[2D40]   SAY "Whaddaya say we just do like the robot says before the laser pulpers kick in ..."  '[skip 1]
[2D68]   rec_06C4 &= !0x2
[2D6E]   SAY "CLICK ON THE RED BUTTON ON THE MAP , SELECT A PLANET AND PUSH THE CONTROL STICK ..."  '[skip 1]
[2D9A]   A0 = 1
[2DA1]   SAY "stop"  '[skip 5]
[2DAB]   POKE [0x27D0] = 0
[2DAF]   POKE [0x2745] = 0
[2DB3]   state[8] = 10
[2DB7]   POKE [0x27D0] = 0
[2DBB]   END PRESENTATION Scruter_K.talk
[2DBE]   BLOCK (exit -> @2DE3)
[2DC2]     GUARD rec_0590 < 2
[2DC9]     GUARD rec_0740 < 2
[2DD0]     GUARD state[8] == 0
[2DD2]     ENDIF
[2DD3]     OP_C3 C3 FC 06 28 00
[2DD8]     T1 = 1
[2DDF]     POKE [0x2DBF] = 0
  END
[2DE3]   BLOCK (exit -> @2F74)
[2DE7]     GUARD T1 == 1
[2DEE]     AWAIT presentation
[2DEF]     GUARD active_actor == Scruter_K.talk (related 40)
[2DF4]     ENDIF
[2DF5]     SAY "MESSAGE RADIO:"
[2E01]     SAY "This is SCRUT agent K , district director . Message to unknown vessel ..."
[2E25]     IF-BLOCK (exit -> @2E6E)
[2E28]       GUARD NOT rec_0F4E == 3380
[2E2E]       ENDIF
[2E2F]       SAY "You deaf or something ?"
[2E41]       SAY "Get out of here ... SWEAR ... INSULT ..."
[2E5B]       SAY "FINAL WARNING"  '[skip 1]
[2E67]       kill = 1
    END
[2E6E]     IF-BLOCK (exit -> @2F44)
[2E71]       GUARD rec_0F4E == 3380
[2E76]       ENDIF
[2E77]       SAY "Next time , you'll really get it ..."
[2E8F]       SAY "You won't have time to be sorry ..."
[2EA7]       SAY "Ha! Ha! Ha! Ha! ... ... ... ... ... ... ... Hurk ! Hurk ! Haurk !"
[2ED1]       SAY "REPORT FROM HONK:"
[2EDF]       SAY "Commander ! Commander ! We really fooled those dummies ... Ha! Ha! Ha! Ha!"
[2F03]       SAY "Hm ! Sorry about that ..."
[2F17]       IF-BLOCK (exit -> @2F44)
[2F1A]         GUARD A1 == 0
[2F21]         ENDIF
[2F22]         SAY "Click on the planet Corpo . The Orxx will be automatically ejected ..."
      END
    END
[2F44]     SAY "..."  '[skip 1]
[2F4E]     rec_06C4 &= !0x2
[2F54]     SAY "stop"  '[skip 5]
[2F5E]     POKE [0x2DE4] = 0
[2F62]     T1 = -1.value
[2F69]     POKE [0x2F75] = 1
[2F6D]     POKE [0x3043] = 1
[2F71]     END PRESENTATION Scruter_K.talk
  END
[2F74]   GOTO @2F93
[2F78]   kill = 1
[2F7F]   state[4] = 43937
[2F83] ?? invalid opcode 00
