import OpenAI from "openai";


async function getResponse(apiKey: string, model: string, content: string) {
    const client = new OpenAI({
        apiKey,
    });

    const stream = await client.chat.completions.create({
        messages: [{ role: 'user', content }],
        model,
        stream: true,
    });

    const responses: string[] = [];
    for await (const chunk of stream) {
        responses.push(chunk.choices[0]?.delta?.content || '');
    }

    return responses.join('');
}

export default { getResponse };
