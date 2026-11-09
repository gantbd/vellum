// Reference verification tool only; not an engine dependency or production provider.
// Word evaluator follows normative UAX #29 revision 49, not ICU's dictionary iterator.
import com.ibm.icu.text.BreakIterator;
import com.ibm.icu.lang.UCharacter;
import com.ibm.icu.lang.UProperty;
import com.ibm.icu.util.ULocale;
import com.ibm.icu.util.VersionInfo;
import static com.ibm.icu.lang.UCharacter.WordBreak.*;
import java.nio.charset.StandardCharsets;
import java.io.*;
import java.util.*;

class UnicodeBoundary {
  static boolean one(int v,int... values){for(int x:values)if(v==x)return true;return false;}
  static boolean ignored(int v){return one(v,EXTEND,FORMAT,ZWJ);}
  static boolean newline(int v){return one(v,CR,LF,NEWLINE);}
  static boolean ah(int v){return one(v,ALETTER,HEBREW_LETTER);}
  static boolean midLetter(int v){return one(v,MIDLETTER,MIDNUMLET,SINGLE_QUOTE);}
  static boolean midNumber(int v){return one(v,MIDNUM,MIDNUMLET,SINGLE_QUOTE);}
  static int prev(int[] p,int i){while(i>=0&&ignored(p[i]))i--;return i;}
  static int next(int[] p,int i){while(i<p.length&&ignored(p[i]))i++;return i;}
  static int at(int[] p,int i){return i<0||i>=p.length?-1:p[i];}
  static boolean wordBreak(int[] cp,int[] p,int i){
    int a=p[i-1],b=p[i];
    if(a==CR&&b==LF)return false;                           // WB3
    if(newline(a)||newline(b))return true;                  // WB3a/b
    if(a==ZWJ&&UCharacter.hasBinaryProperty(cp[i],UProperty.EXTENDED_PICTOGRAPHIC))return false; // WB3c
    if(a==WSEGSPACE&&b==WSEGSPACE)return false;              // WB3d
    if(ignored(b))return false;                            // WB4 (after newline handled above)
    int left=prev(p,i-1),right=i;
    a=at(p,left);b=at(p,right);
    int before=at(p,prev(p,left-1)),after=at(p,next(p,right+1));
    if(ah(a)&&ah(b))return false;                          // WB5
    if(ah(a)&&midLetter(b)&&ah(after))return false;         // WB6
    if(ah(before)&&midLetter(a)&&ah(b))return false;        // WB7
    if(a==HEBREW_LETTER&&b==SINGLE_QUOTE)return false;       // WB7a
    if(a==HEBREW_LETTER&&b==DOUBLE_QUOTE&&after==HEBREW_LETTER)return false; // WB7b
    if(before==HEBREW_LETTER&&a==DOUBLE_QUOTE&&b==HEBREW_LETTER)return false; // WB7c
    if(a==NUMERIC&&b==NUMERIC||ah(a)&&b==NUMERIC||a==NUMERIC&&ah(b))return false; // WB8/9/10
    if(before==NUMERIC&&midNumber(a)&&b==NUMERIC)return false; // WB11
    if(a==NUMERIC&&midNumber(b)&&after==NUMERIC)return false; // WB12
    if(a==KATAKANA&&b==KATAKANA)return false;                // WB13
    if((ah(a)||one(a,NUMERIC,KATAKANA,EXTENDNUMLET))&&b==EXTENDNUMLET)return false; // WB13a
    if(a==EXTENDNUMLET&&(ah(b)||one(b,NUMERIC,KATAKANA)))return false; // WB13b
    if(a==REGIONAL_INDICATOR&&b==REGIONAL_INDICATOR){
      int count=0;for(int j=left;j>=0&&p[j]==REGIONAL_INDICATOR;j=prev(p,j-1))count++;
      if(count%2==1)return false;                          // WB15/16
    }
    return true;                                          // WB999
  }
  static List<Integer> words(String s){
    int[] cp=s.codePoints().toArray(),p=Arrays.stream(cp).map(c->UCharacter.getIntPropertyValue(c,UProperty.WORD_BREAK)).toArray();
    List<Integer> out=new ArrayList<>();out.add(0);int offset=0;
    for(int i=1;i<cp.length;i++){offset+=Character.charCount(cp[i-1]);if(wordBreak(cp,p,i))out.add(offset);}
    if(!s.isEmpty())out.add(s.length());return out;
  }
  static List<Integer> iterated(String s,boolean word){
    BreakIterator bi=word?BreakIterator.getWordInstance(ULocale.ROOT):BreakIterator.getCharacterInstance(ULocale.ROOT);
    bi.setText(s);List<Integer> out=new ArrayList<>();for(int p=bi.first();p!=BreakIterator.DONE;p=bi.next())out.add(p);return out;
  }
  static int byteOffset(String s,int utf16){
    if(utf16>0&&utf16<s.length()&&Character.isLowSurrogate(s.charAt(utf16))&&Character.isHighSurrogate(s.charAt(utf16-1)))throw new AssertionError("split surrogate");
    return s.substring(0,utf16).getBytes(StandardCharsets.UTF_8).length;
  }
  static List<Integer> bytes(String s,List<Integer> offsets){return offsets.stream().map(p->byteOffset(s,p)).toList();}
  static String evidence(String s){
    List<Integer> w=words(s);List<List<Integer>> significant=new ArrayList<>();
    for(int i=1;i<w.size();i++){
      String part=s.substring(w.get(i-1),w.get(i));
      boolean selected=part.codePoints().anyMatch(c->{int t=UCharacter.getType(c);return UCharacter.isUAlphabetic(c)||one(t,9,10,11);});
      if(selected)significant.add(List.of(byteOffset(s,w.get(i-1)),byteOffset(s,w.get(i))));
    }
    return "["+bytes(s,iterated(s,false))+","+bytes(s,w)+","+significant+","+bytes(s,iterated(s,true))+"]";
  }
  public static void main(String[] args)throws Exception{
    if(!VersionInfo.ICU_VERSION.toString().equals("79.1.0.0")||!UCharacter.getUnicodeVersion().toString().equals("18.0.0.0"))throw new AssertionError("wrong Unicode/provider version");
    System.out.println("{\"icu\":\"79.1\",\"unicode\":\"18.0.0\",\"uax29\":49}");
    try(BufferedReader in=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.US_ASCII))){
      String line;int count=0;
      while((line=in.readLine())!=null){
        if(++count>4096||line.length()>8192)throw new AssertionError("tool batch bound");
        byte[] raw=Base64.getDecoder().decode(line);String s=new String(raw,StandardCharsets.UTF_8);
        if(!Arrays.equals(raw,s.getBytes(StandardCharsets.UTF_8))||s.codePointCount(0,s.length())>2048)throw new AssertionError("tool scalar/input bound");
        System.out.println(evidence(s));
      }
    }
  }
}
