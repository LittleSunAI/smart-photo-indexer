import type { AiProvider, ImageAnalysisRequest, ImageAnalysisResult } from './AiProvider';
import { normalizeAiTags } from './AiProvider';

interface MockScenario {
  keywords: string[];
  descriptionRu: string;
  descriptionEn: string;
  tags: string[];
  objects: string[];
  scene: string;
}

const mockScenarios: MockScenario[] = [
  {
    keywords: ['birthday', 'cake', 'party', 'celebration', 'день', 'рожд'],
    descriptionRu: 'На фотографии праздничная сцена: люди собрались рядом со столом, виден торт и атмосфера дня рождения. Изображение подходит для поиска по словам: праздник, торт, семья, друзья, застолье.',
    descriptionEn: 'A birthday celebration scene with people gathered near a table and a cake. The image is suitable for searches like party, cake, family, friends, and celebration.',
    tags: ['birthday', 'cake', 'party', 'people', 'table', 'celebration', 'family', 'friends'],
    objects: ['cake', 'table', 'people', 'decorations'],
    scene: 'indoor celebration',
  },
  {
    keywords: ['dog', 'pet', 'cat', 'pets', 'animal', 'собака', 'кот'],
    descriptionRu: 'На фотографии домашнее животное. Сцена подходит для поиска по словам: питомец, животное, дом, прогулка, семейные фото.',
    descriptionEn: 'A pet or animal photo. The scene is useful for searches like pet, animal, home, walk, and family photos.',
    tags: ['pet', 'animal', 'home', 'family', 'cute'],
    objects: ['animal'],
    scene: 'pet photo',
  },
  {
    keywords: ['mountain', 'lake', 'forest', 'sunset', 'pier', 'trip', 'travel', 'landscape', 'горы', 'лес', 'закат'],
    descriptionRu: 'На фотографии пейзаж или кадр из поездки: природа, открытое пространство и спокойная атмосфера. Подходит для поиска по словам: путешествие, природа, пейзаж, прогулка.',
    descriptionEn: 'A landscape or travel photo with nature, open space, and a calm atmosphere. Useful for searches like travel, nature, landscape, and walk.',
    tags: ['landscape', 'travel', 'nature', 'outdoor', 'scenery'],
    objects: ['sky', 'nature'],
    scene: 'outdoor landscape',
  },
  {
    keywords: ['family', 'friends', 'group', 'people', 'portrait', 'семья', 'друзья', 'люди'],
    descriptionRu: 'На фотографии группа людей или семейный кадр. Изображение удобно искать по словам: люди, семья, друзья, портрет, встреча.',
    descriptionEn: 'A group of people or a family portrait. The image is useful for searches like people, family, friends, portrait, and gathering.',
    tags: ['people', 'family', 'friends', 'group', 'portrait'],
    objects: ['people'],
    scene: 'people gathering',
  },
];

const fallbackScenario: MockScenario = {
  keywords: [],
  descriptionRu: 'MockProvider выполнил тестовый AI-анализ изображения. Это временное описание создано без отправки фото во внешний сервис и нужно для проверки очереди, SQLite и интерфейса.',
  descriptionEn: 'MockProvider completed a test AI analysis for this image. This temporary description was generated without sending the photo to an external service and is used to test the queue, SQLite, and UI.',
  tags: ['mock-analysis', 'image', 'test', 'indexed'],
  objects: ['image'],
  scene: 'mock image analysis',
};

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function pickScenario(fileName: string): MockScenario {
  const normalized = fileName.toLowerCase();
  return mockScenarios.find((scenario) => scenario.keywords.some((keyword) => normalized.includes(keyword))) ?? fallbackScenario;
}

export class MockProvider implements AiProvider {
  readonly id = 'mock' as const;
  readonly displayName = 'Mock AI Provider';

  async analyzeImage(request: ImageAnalysisRequest): Promise<ImageAnalysisResult> {
    await delay(350);
    const scenario = pickScenario(request.fileName);
    const description = request.language === 'ru' ? scenario.descriptionRu : scenario.descriptionEn;

    return {
      description,
      tags: normalizeAiTags([...scenario.tags, request.providerMode, request.model === 'mock/test-vision' ? 'mock-provider' : request.model]),
      objects: scenario.objects,
      scene: scenario.scene,
      confidence: scenario === fallbackScenario ? 0.72 : 0.86,
      source: 'mock',
      model: request.model || 'mock/test-vision',
      analyzedAt: new Date().toISOString(),
      rawText: JSON.stringify({ provider: 'mock', fileName: request.fileName, scene: scenario.scene }),
    };
  }
}
